use dynasmrt::{AssemblyOffset, DynasmApi, DynasmLabelApi, ExecutableBuffer};
use rustc_hash::FxHashMap;

use super::analysis::{analyze_block_stack, ReturnCode, StackOps};
use super::emit::{emit, Context};
use super::interpreter::WRAPPER;
use crate::il2cpp_array::Il2CppArray;
use crate::udon_types::UdonHeap;

pub struct Dynarec {
    bytecode: Vec<u32>,
    pub pc: u32,
    buffer: ExecutableBuffer,
    block_cache: FxHashMap<u32, AssemblyOffset>,
    state: Context,
}

extern "C" {
    // our generated code doesn't actually touch the struct, it treats it like an opaque pointer.
    #[allow(improper_ctypes)]
    fn wrap_vm_exception_unknown(code_ptr: *const u8, state: &mut Context) -> u64;
}

impl Dynarec {
    pub fn new(array: &Il2CppArray<u8>, heap: &'static mut UdonHeap) -> Option<Self> {
        let bytes = array.as_slice();
        let bytecode: Vec<u32> = bytes
            .chunks_exact(4)
            .map(|chunk| u32::from_be_bytes(chunk.try_into().unwrap()))
            .collect();

        let mut jump_targets = Vec::<u32>::new();
        let mut block_ops = FxHashMap::default();

        let mut pc = 0usize;
        while pc / 4 < bytecode.len() {
            let start_pc = pc as u32;

            let stack_ops = analyze_block_stack(&bytecode, heap, &mut pc, unsafe { WRAPPER });

            match stack_ops.last() {
                Some(StackOps::Return(ReturnCode::Continue(_))) => {}
                Some(StackOps::Return(reason)) => {
                    println!("Error parsing bytecode: {:?}", reason);
                    return None;
                }
                _ => {}
            }

            block_ops.insert(start_pc, stack_ops);
            jump_targets.push(start_pc);
        }

        while let Some(destination) = jump_targets.pop() {
            let targets: Vec<(u32, Vec<StackOps>)> = block_ops[&destination]
                .iter()
                .map(|op| match op {
                    StackOps::Jump(destination) => Some(*destination as u32),
                    StackOps::JumpIfFalse { destination, .. } => Some(*destination),
                    _ => None,
                })
                .filter(|destination| destination.is_some())
                .map(|destination| destination.unwrap())
                .filter(|destination| !block_ops.contains_key(destination))
                .map(|destination| {
                    let mut pc = destination as usize;
                    let stack_ops =
                        analyze_block_stack(&bytecode, heap, &mut pc as &mut usize, unsafe {
                            WRAPPER
                        });
                    (destination, stack_ops)
                })
                .collect();

            for (destination, stack_ops) in targets {
                block_ops.insert(destination, stack_ops);
                jump_targets.push(destination);
            }
        }

        let mut assembler = dynasmrt::x64::Assembler::new().ok()?;
        let labels = FxHashMap::from(
            block_ops
                .keys()
                .map(|&target| {
                    let label = assembler.new_dynamic_label();
                    (target, label)
                })
                .collect(),
        );

        let mut block_cache = FxHashMap::default();

        for (pc, ops) in block_ops {
            let label = labels.get(&pc).unwrap();
            assembler.dynamic_label(*label);
            block_cache.insert(pc, assembler.offset());
            emit(&mut assembler, &ops, &labels);
        }

        let state = Context::new(heap as *mut UdonHeap);

        Some(Dynarec {
            bytecode,
            pc: 0,
            buffer: assembler.finalize().unwrap(),
            block_cache,
            state: state,
        })
    }

    pub fn interpret(&mut self) -> bool {
        loop {
            let offset = self.block_cache[&self.pc];

            let rc = ReturnCode::decode(unsafe {
                wrap_vm_exception_unknown(self.buffer.ptr(offset), &mut self.state)
            });

            match rc {
                ReturnCode::Continue(pc) => {
                    self.pc = pc;
                    if (pc / 4) as usize >= self.bytecode.len() {
                        return true;
                    }
                    continue;
                }
                ReturnCode::RequestInterpreter(pc) => {
                    println!("old pc: {}, new pc: {}", self.pc, pc);
                    // self.pc = pc; // we don't actually emit the correct pc at the moment! but it doesn't matter, either.

                    println!("dumping blocks");
                    // for (pc, block) in self.block_cache.iter() {
                    //     std::fs::write(format!("block_{:x}.bin", pc), block.as_ref()).unwrap();
                    // }
                    std::fs::write(format!("block_{:x}.bin", pc), self.buffer.as_ref()).unwrap();
                    unimplemented!();
                }
                ReturnCode::MissingArgument => return false,
                ReturnCode::UnknownOpCode(_op) => return false,
                ReturnCode::StackUnderflow => return false,
                ReturnCode::UnknownReturn(_ret) => return false,
            }
        }
    }
}

/* API */
use std::{
    alloc::{dealloc, Layout},
    ptr::drop_in_place,
};

#[no_mangle]
pub extern "C" fn dynarec_load_program(
    array: &Il2CppArray<u8>,
    heap: &'static mut UdonHeap,
) -> *mut Dynarec {
    if let Some(dynarec) = Dynarec::new(array, heap) {
        Box::into_raw(Box::new(dynarec))
    } else {
        std::ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn dynarec_interpret(dynarec: &mut Dynarec) -> bool {
    dynarec.interpret()
}

#[no_mangle]
pub extern "C" fn dynarec_set_program_counter(dynarec: &mut Dynarec, pc: u32) -> bool {
    if pc % 4 != 0 || pc / 4 >= dynarec.bytecode.len() as u32 {
        return false;
    }
    dynarec.pc = pc;
    true
}

#[no_mangle]
pub extern "C" fn dynarec_get_program_counter(dynarec: &Dynarec) -> u32 {
    dynarec.pc as u32
}

#[no_mangle]
pub extern "C" fn dynarec_dispose(dynarec: *mut Dynarec) {
    unsafe {
        drop_in_place(dynarec);
        dealloc(dynarec as *mut u8, Layout::new::<Dynarec>());
    }
}
