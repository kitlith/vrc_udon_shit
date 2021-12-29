
use num_traits::FromPrimitive;

use super::OpCode;
use crate::il2cpp_object::Il2CppObject;
use crate::il2cpp_string::Il2CppString;
use crate::udon_types::{UdonHeap, UdonWrapperCallbackType, IUdonWrapper};

#[derive(Debug, Clone)]
pub enum ExternArgs {
    Complete(Vec<u32>),
    Incomplete(usize),
}

impl ExternArgs {
    pub fn len(&self) -> usize {
        match self {
            ExternArgs::Complete(a) => a.len(),
            ExternArgs::Incomplete(c) => *c
        }
    }
}

#[derive(Debug, Clone)]
pub enum ReturnCode {
    // semantics of Continue are: detect if should halt, find next block, compile next block if necessary.
    Continue(u32),
    RequestInterpreter(u32), // NOTE: currently only used for stack underflow condition.
    MissingArgument, // ERROR: Unexpected EOF in middle of instruction
    UnknownOpCode(u32), // ERROR: Unknown opcode {this.0}
    StackUnderflow, // ERROR: tried to pop item when stack was empty!
    UnknownReturn(u64)
}

impl ReturnCode {
    pub fn encode(&self) -> u64 {
        match self {
            ReturnCode::Continue(pc) => *pc as u64,
            ReturnCode::RequestInterpreter(pc) => *pc as u64 | (1 << 63),
            ReturnCode::MissingArgument => (0x00000001 << 32),
            ReturnCode::UnknownOpCode(op) => (0x00000002 << 32) | *op as u64,
            ReturnCode::StackUnderflow => (0x00000003 << 32),
            ReturnCode::UnknownReturn(unk) => *unk,
        }
    }

    pub fn decode(val: u64) -> Self {
        match val >> 32 {
            0x00000000 => ReturnCode::Continue(val as u32),
            0x80000000 => ReturnCode::RequestInterpreter(val as u32),
            0x00000001 => ReturnCode::MissingArgument,
            0x00000002 => ReturnCode::UnknownOpCode(val as u32),
            0x00000003 => ReturnCode::StackUnderflow,
            _ => ReturnCode::UnknownReturn(val),
        }
    }
}

#[derive(Clone)]
pub enum StackOps {
    Push(Vec<u32>),
    Pop(usize),
    Extern {
        callback: UdonWrapperCallbackType,
        target: &'static Il2CppObject,
        // keep track of the original index in case we need to invalidate the block.
        heap_slot: u32,
        args: ExternArgs
    },
    // can be implemented as a return, 
    JumpIfFalse {
        destination: u32,
        arg: Option<u32>
    },
    CopyComplete {
        src: u32,
        dst: u32
    },
    CopyIncomplete,
    // can be implemented as a return, possible optimization is to jump directly to next block
    Jump(usize),
    // will be implemented as a return, possible optimization to query block cache and attempt to dispatch next block.
    JumpIndirect(u32),
    // exit vm code, return to handler.
    Return(ReturnCode),
}

pub fn analyze_block_stack(bytecode: &[u32], heap: &UdonHeap, ext_pc: &mut usize, wrapper: *const IUdonWrapper) -> Vec<StackOps> {
    let mut stack = Vec::<u32>::new();
    let mut ops = Vec::<StackOps>::new();

    let mut pc = *ext_pc;

    fn flush_stack(stack: &mut Vec<u32>, ops: &mut Vec<StackOps>) {
        if stack.len() != 0 {
            let items = std::mem::replace(stack, Vec::new());
            ops.push(StackOps::Push(items));
        }
    }

    let terminating_op = loop {
        macro_rules! get_or_halt {
            ($index:expr, $reason:expr) => {
                if let Some(&opcode) = bytecode.get($index) {
                    opcode
                } else {
                    break StackOps::Return($reason);
                }
            };
            ($index:expr) => { get_or_halt!($index, ReturnCode::MissingArgument) };
        }
        let opcode = get_or_halt!(pc >> 2, ReturnCode::Continue(pc as u32));

        match FromPrimitive::from_u32(opcode) {
            Some(OpCode::Nop) => { pc += 4; }
            Some(OpCode::Annotation) => { pc += 8; }
            Some(OpCode::Push) => {
                let heap_slot = get_or_halt!((pc >> 2) + 1);
                stack.push(heap_slot);
                pc += 8;
            }
            Some(OpCode::Pop) => {
                if stack.pop().is_none() {
                    if let Some(StackOps::Pop(count)) = ops.last_mut() {
                        *count += 1;
                    } else {
                        ops.push(StackOps::Pop(1));
                    }
                }
                pc += 4;
            }
            Some(OpCode::JumpIfFalse) => {
                // TODO: do we want to avoid flushing stack here, and instead set it up to only push stack if the branch is taken?
                flush_stack(&mut stack, &mut ops);

                ops.push(StackOps::JumpIfFalse {
                    destination: get_or_halt!((pc >> 2) + 1),
                    arg: stack.pop()
                });
                
                pc += 8;
            }
            Some(OpCode::Jump) => {
                // TODO: detect jump past end of bytecode/to 0xFFFFFFFF as halt?
                let dest = get_or_halt!((pc >> 2) + 1) as usize;

                pc += 8;

                // ends the block
                break if dest == 0xFFFFFFFF || dest == 0xFFFFFFFC {
                    StackOps::Return(ReturnCode::Continue(dest as u32))
                } else {
                    StackOps::Jump(dest)
                };
            }
            Some(OpCode::Extern) => {
                let heap_slot = get_or_halt!((pc >> 2) + 1);
                let signature: &Il2CppString = (*heap).get_object(heap_slot);
                let method = unsafe { (*wrapper).get_extern_function_delegate(signature) };
                let parameter_count = unsafe { (*wrapper).get_extern_function_parameter_count(signature) as usize };

                let op = StackOps::Extern {
                    callback: method.method_ptr,
                    target: method.m_target,
                    heap_slot,
                    args: if stack.len() >= parameter_count {
                        ExternArgs::Complete(stack.split_off(stack.len()-parameter_count))
                    } else {
                        flush_stack(&mut stack, &mut ops);
                        ExternArgs::Incomplete(parameter_count)
                    }
                };

                ops.push(op);

                pc += 8;
            }
            Some(OpCode::JumpIndirect) => {
                let heap_slot = get_or_halt!((pc >> 2) + 1);
                pc += 8;
                break StackOps::JumpIndirect(heap_slot); // ends the block
            }
            Some(OpCode::Copy) => {
                if stack.len() >= 2 {
                    let dst = stack.pop().unwrap();
                    let src = stack.pop().unwrap();
                    ops.push(StackOps::CopyComplete { src, dst });
                } else {
                    flush_stack(&mut stack, &mut ops);
                    ops.push(StackOps::CopyIncomplete);
                }
                pc += 4;
            }
            // NOTE: only really applicable if we have a mix of running interpreter and recompiling
            #[allow(unreachable_code, unused_variables)]
            Some(OpCode::CachedExtern) => {
                let heap_slot = get_or_halt!((pc >> 2) + 1);
                let parameter_count = unimplemented!(); // TODO: should just have access to the interpreter state.

                // ops.push(StackOps::Extern {
                //     heap_slot,
                //     args: if stack.len() >= parameter_count {
                //         ExternArgs::Complete(stack.split_off(stack.len()-parameter_count))
                //     } else {
                //         ExternArgs::Incomplete(parameter_count)
                //     }
                // });

                pc += 8;
            }
            None => { 
                break StackOps::Return(ReturnCode::UnknownOpCode(opcode));
            },
        }
    };

    flush_stack(&mut stack, &mut ops);

    ops.push(terminating_op);

    *ext_pc = pc;

    //println!("ops: {:?}", ops);

    ops
}