use dynasmrt::ExecutableBuffer;
use rustc_hash::FxHashMap;

use super::emit::{emit, Context};
use super::interpreter::WRAPPER;
use super::analysis::{analyze_block_stack, ReturnCode, StackOps};
use crate::udon_types::UdonHeap;
use crate::il2cpp_array::Il2CppArray;

pub struct Dynarec {
    heap: &'static mut UdonHeap,
    bytecode: Vec<u32>,
    pub pc: u32,
    // pushed_variables: Vec<(i32, i32)>,
    block_cache: FxHashMap<u32, ExecutableBuffer>,
    state: Context,
}

extern "C" {
    // our generated code doesn't actually touch the struct, it treats it like an opaque pointer.
    #[allow(improper_ctypes)]
    fn wrap_vm_exception_unknown(
        code_ptr: *const u8,
        state: &mut Context,
    ) -> u64;
}

impl Dynarec {
    pub fn new(array: &Il2CppArray<u8>, heap: &'static mut UdonHeap) -> Option<Self> {
        // let heap_size = heap.size();
        // let mut pushed_variables = vec![(0u32, u32::MAX); heap_size];

        let bytes = array.as_slice();
        let bytecode: Vec<u32> = bytes
            .chunks_exact(4)
            .map(|chunk| u32::from_be_bytes(chunk.try_into().unwrap()))
            .collect();

        let mut jump_targets = Vec::<u32>::new();
        let mut block_cache = FxHashMap::default();

        let mut pc = 0usize;
        while pc / 4 < bytecode.len() {
            let start_pc = pc;
            let stack_ops = analyze_block_stack(&bytecode, heap, &mut pc, unsafe { WRAPPER });

            match stack_ops.last() {
                Some(StackOps::Return(ReturnCode::Continue(_))) => {}
                Some(StackOps::Return(reason)) => {
                    println!("Error parsing bytecode: {:?}", reason);
                    return None;
                }
                _ => {}
            }

            for op in stack_ops.iter() {
                match op {
                    StackOps::Jump(destination) => Some(*destination as u32),
                    StackOps::JumpIfFalse { destination, .. } => Some(*destination),
                    _ => None,
                }.map(|dest| if !block_cache.contains_key(&dest) { jump_targets.push(dest); });
            }

            block_cache.insert(start_pc as u32, emit(&stack_ops));
        }

        while let Some(destination) = jump_targets.pop() {
            if block_cache.contains_key(&destination) {
                continue;
            }

            let mut pc = destination as usize;
            let stack_ops = analyze_block_stack(&bytecode, heap, &mut pc, unsafe { WRAPPER });
            for op in stack_ops.iter() {
                match op {
                    StackOps::Jump(destination) => Some(*destination as u32),
                    StackOps::JumpIfFalse { destination, .. } => Some(*destination),
                    _ => None,
                }.map(|dest| if !block_cache.contains_key(&dest) { jump_targets.push(dest); });
            }

            block_cache.insert(destination as u32, emit(&stack_ops));
        }

        let state = Context::new(heap as *mut UdonHeap);

        Some(Dynarec {
            heap,
            bytecode,
            pc: 0,
            // pushed_variables,
            block_cache,
            state: state,
        })
    }

    pub fn interpret(&mut self) -> bool {
        loop {
            let block = self.block_cache.entry(self.pc)
                .or_insert_with(|| {
                    let mut end_pc = self.pc as usize;
                    let stack_ops = analyze_block_stack(&self.bytecode, self.heap, &mut end_pc, unsafe { WRAPPER });
                    let block = emit(&stack_ops);
                    block
                });
            
            let rc = ReturnCode::decode(unsafe { wrap_vm_exception_unknown(
                block.as_ptr(),
                &mut self.state
            ) });
            
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
                    for (pc, block) in self.block_cache.iter() {
                        std::fs::write(format!("block_{:x}.bin", pc), block.as_ref()).unwrap();
                    }
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
