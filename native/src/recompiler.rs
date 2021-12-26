use core::arch::asm;
use num_traits::FromPrimitive;

use crate::interpreter::OpCode;
//use crate::il2cpp_object::Il2CppObject;
use crate::il2cpp_string::Il2CppString;
use crate::udon_types::{UdonHeap, UdonWrapperCallbackType, IUdonWrapper};

pub struct State {
    heap_ptr: *const core::ffi::c_void, // r12 / x19
    stack_ptr: *mut core::ffi::c_void,  // r13 / x20
}

extern "C" {
    // meant to be jumped to by jitted assembly!
    //fn vm_code_return();
}

// NOTE: This has undefined behaviour! C++ exceptions are not supposed to unwind through rust functions.
// this seems to work anyway on windows? so why the fuck not.
#[no_mangle]
#[inline(never)] // we use a named label, cannot allow it to conflict.
#[allow(named_asm_labels)]
pub extern "C" fn call_vm_code(code_ptr: *const core::ffi::c_void, state: &mut State) -> u64 {
    let ret: u64;

    #[cfg(target_arch = "x86_64")]
    unsafe {
        // our generated code should not touch the stack except for making function calls.
        // essentially, a fancy function body that isn't in the same place as the rest of the body.
        asm!(
            "jmp rcx",
            ".global vm_code_return",
            // TODO: consider using a local label, and putting its address in a register?
            "vm_code_return:",
            // we don't really care which register this goes in
            // but as the first arg it should already be in rcx
            inout("rcx") code_ptr => _,
            inout("r12") state.heap_ptr,
            inout("r13") state.stack_ptr,
            out("rax") ret,
            // TODO: just specify system, as that's what it'll be on windows anyway?
            clobber_abi("win64"),
        );
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!(
            "b x0",
            ".global vm_code_return",
            "vm_code_return:",
            // don't really care, but first argument/result should already be in this register.
            inout("x0") code_ptr => ret,
            inout("x19") state.heap_ptr,
            inout("x20") state.stack_ptr,
            clobber_abi("system")
        );
    }
    // lower 32 bits should be bytecode PC, upper 32 bits should signal if there's an error/exception
    ret
}

#[derive(Debug)]
enum ExternArgs {
    Complete(Vec<u32>),
    Incomplete(usize)
}

#[derive(Debug)]
enum StackOps {
    Push(Vec<u32>),
    Pop(usize),
    Extern {
        // callback: UdonWrapperCallbackType,
        // target: &'static Il2CppObject,
        // keep track of the original index in case we need to invalidate the block.
        heap_slot: u32,
        args: ExternArgs
    },
    JumpIfFalse {
        destination: u32,
        arg: Option<u32>
    },
    CopyComplete {
        src: u32,
        dst: u32
    },
    CopyIncomplete,
    Jump(usize),
    JumpIndirect(u32)
}

fn analyze_block_stack(bytecode: &[u32], heap: &UdonHeap, mut pc: usize, wrapper: *const IUdonWrapper) -> Result<Vec<StackOps>, ()> {
    let mut stack = Vec::<u32>::new();
    let mut ops = Vec::<StackOps>::new();
    while let Some(&opcode) = bytecode.get(pc >> 2) {
        match FromPrimitive::from_u32(opcode) {
            Some(OpCode::Nop) => { pc += 4; }
            Some(OpCode::Annotation) => { pc += 8; }
            Some(OpCode::Push) => {
                let heap_slot = *bytecode.get((pc >> 2) + 1).ok_or(())?;
                stack.push(heap_slot);
                pc += 8;
            }
            Some(OpCode::Pop) => {
                // TODO: coalesce, so long as it is not separated by other incomplete ops.
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
                ops.push(StackOps::JumpIfFalse {
                    destination: *bytecode.get((pc >> 2) + 1).ok_or(())?,
                    arg: stack.pop()
                });
                
                pc += 8;
            }
            Some(OpCode::Jump) => {
                // TODO: push onto ops?
                ops.push(StackOps::Push(stack.clone()));
                let dest = *bytecode.get((pc >> 2) + 1).ok_or(())? as usize;
                ops.push(StackOps::Jump(dest));
                break; // ends the block
            }
            Some(OpCode::Extern) => {
                let heap_slot = *bytecode.get((pc >> 2) + 1).ok_or(())?;
                let signature: &Il2CppString = (*heap).get_object(heap_slot);
                let parameter_count = unsafe { (*wrapper).get_extern_function_parameter_count(signature) as usize };

                ops.push(StackOps::Extern {
                    heap_slot,
                    args: if stack.len() >= parameter_count {
                        ExternArgs::Complete(stack.split_off(stack.len()-parameter_count))
                    } else {
                        ExternArgs::Incomplete(parameter_count)
                    }
                });

                pc += 8;
            }
            Some(OpCode::JumpIndirect) => {
                ops.push(StackOps::Push(stack.clone()));
                let heap_slot = *bytecode.get(pc / 4 + 1).ok_or(())?;
                ops.push(StackOps::JumpIndirect(heap_slot));
                break; // ends the block
            }
            Some(OpCode::Copy) => {
                if stack.len() >= 2 {
                    let dst = stack.pop().unwrap();
                    let src = stack.pop().unwrap();
                    ops.push(StackOps::CopyComplete { src, dst });
                } else {
                    ops.push(StackOps::CopyIncomplete);
                }
                pc += 4;
            }
            #[allow(unreachable_code, unused_variables)]
            Some(OpCode::CachedExtern) => {
                let heap_slot = *bytecode.get(pc / 4 + 1).ok_or(())?;
                let parameter_count = unimplemented!(); // TODO: should just have access to the interpreter state.

                ops.push(StackOps::Extern {
                    heap_slot,
                    args: if stack.len() >= parameter_count {
                        ExternArgs::Complete(stack.split_off(stack.len()-parameter_count))
                    } else {
                        ExternArgs::Incomplete(parameter_count)
                    }
                });

                pc += 8;
            }
            _ => { 
                println!("Unknown opcode: {0}", opcode);
                return Err(());
            },
        }
    }

    println!("ops: {:?}", ops);
    if stack.len() != 0 {
        println!("Extra stack count: {}", stack.len());
    }

    Ok(ops)
}