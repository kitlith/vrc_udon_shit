use crate::il2cpp_array::Il2CppArray;
use crate::il2cpp_object::Il2CppObject;
use crate::il2cpp_string::Il2CppString;
use crate::span::Span;
use crate::udon_types::{
    IUdonWrapper, UdonHeap, UdonVMTimeSource, UdonWrapperCallbackType,
};

use super::OpCode;

use num_traits::FromPrimitive;
use rustc_hash::FxHashMap;

const INITIAL_STACK_SIZE: usize = 1024;

pub struct InterpreterState {
    pc: usize,
    stack: Vec<u32>,
    bytecode: Vec<u32>,
    heap: &'static mut UdonHeap,
    extern_cache: FxHashMap<u32, (UdonWrapperCallbackType, &'static Il2CppObject, i32)>,
}

pub static mut WRAPPER: *const IUdonWrapper = 0 as *const IUdonWrapper;
static mut TIME_SOURCE: *const UdonVMTimeSource = 0 as *const UdonVMTimeSource;

#[no_mangle]
pub extern "C" fn set_wrapper_and_time_source(
    wrapper: &IUdonWrapper,
    time_source: &UdonVMTimeSource,
) {
    unsafe {
        WRAPPER = wrapper as *const IUdonWrapper;
        TIME_SOURCE = time_source as *const UdonVMTimeSource;
    }
}

impl InterpreterState {
    pub fn new(array: &Il2CppArray<u8>, heap: &'static mut UdonHeap) -> Option<Self> {
        if array.len() % 4 != 0 {
            return None;
        }

        let bytes = array.as_slice();
        let bytecode: Vec<u32> = bytes
            .chunks_exact(4)
            .map(|chunk| u32::from_be_bytes(chunk.try_into().unwrap()))
            .collect();
        
        // println!("array.len() {0}", array.len());
        // println!("bytecode.len() {0}", bytecode.len());

        Some(InterpreterState {
            pc: 0,
            stack: Vec::with_capacity(INITIAL_STACK_SIZE),
            bytecode: bytecode,
            heap: heap,
            extern_cache: FxHashMap::default(),
        })
    }

    pub fn interpret(&mut self) -> Option<()> {
        while let Some(&opcode) = self.bytecode.get(self.pc / 4) {
            match FromPrimitive::from_u32(opcode) {
                Some(OpCode::Nop) => {
                    // println!("OpCode::Nop");
                    self.pc += 4;
                }
                Some(OpCode::Annotation) => {
                    // println!("OpCode::Annotation");
                    // there is an extra argument here we could take advantage of, if we cared.
                    self.pc += 8;
                }
                Some(OpCode::Push) => {
                    // println!("OpCode::Push");
                    let &address = self.bytecode.get(self.pc / 4 + 1)?;
                    self.stack.push(address);
                    self.pc += 8;
                }
                Some(OpCode::Pop) => {
                    // println!("OpCode::Pop");
                    self.stack.pop();
                    self.pc += 4;
                }
                Some(OpCode::JumpIfFalse) => {
                    // println!("OpCode::JumpIfFalse");
                    let &jump_target = self.bytecode.get(self.pc / 4 + 1)?;
                    let cond_slot = self.stack.pop()?;

                    if self.heap.get_value(cond_slot) {
                        self.pc += 8;
                    } else {
                        self.pc = jump_target as usize;
                    }
                }
                Some(OpCode::Jump) => {
                    // println!("OpCode::Jump");
                    let &jump_target = self.bytecode.get(self.pc / 4 + 1)?;
                    self.pc = jump_target as usize;
                }
                Some(OpCode::Extern) => {
                    // println!("OpCode::Extern");
                    let &address = self.bytecode.get(self.pc / 4 + 1)?;

                    unsafe {
                        let signature: &Il2CppString = (*self.heap).get_object(address);
                        // println!("signature {}, {}", signature, signature.len());
                        let method = (*WRAPPER)
                            .get_extern_function_delegate(signature);
                        let parameter_count =
                            (*WRAPPER).get_extern_function_parameter_count(signature);

                        // println!("parameter_count {0}", parameter_count);
                        self.extern_cache.insert(address, (method.method_ptr, method.m_target, parameter_count));
                        // self.heap.set_raw(address, self.extern_cache.get(&address)? as *const _ as *const c_void);
                        self.bytecode[self.pc / 4] = OpCode::CachedExtern as u32;
                        self.invoke(method.method_ptr, method.m_target, parameter_count);
                    }
                    self.pc += 8;
                }
                Some(OpCode::JumpIndirect) => {
                    // println!("OpCode::JumpIndirect");
                    let &address = self.bytecode.get(self.pc / 4 + 1)?;
                    let jump_target: u32 = self.heap.get_value(address);
                    self.pc = jump_target as usize;
                }
                Some(OpCode::Copy) => {
                    // println!("OpCode::Copy");
                    let dst = self.stack.pop()?;
                    let src = self.stack.pop()?;
                    self.heap.copy_variables(src, dst);
                    self.pc += 4;
                }
                Some(OpCode::CachedExtern) => {
                    // println!("OpCode::CachedExtern");
                    let &address = self.bytecode.get(self.pc / 4 + 1)?;
                    let &(method, target, parameter_count) = self.extern_cache.get(&address)?;
                    self.invoke(method, target, parameter_count);
                    self.pc += 8;
                }
                _ => { 
                    println!("Unknown opcode: {0}", opcode);
                    return None;
                },
            }
        }

        Some(())
    }

    fn invoke(&mut self, method: UdonWrapperCallbackType, target: &Il2CppObject, parameter_count: i32) {
        let start = self.stack.len() - parameter_count as usize;
        let slice = &self.stack[start..];
        let span = Span::new(slice);
        method(target, self.heap, &span);
        self.stack.truncate(start);
    }

    pub fn set_pc(&mut self, pc: usize) -> Option<()> {
        if pc % 4 != 0 || pc / 4 >= self.bytecode.len() {
            return None;
        }
        self.pc = pc;
        Some(())
    }
}

/* API */
use std::{
    alloc::{dealloc, Layout},
    ptr::drop_in_place,
};

#[no_mangle]
pub extern "C" fn load_program(array: &Il2CppArray<u8>, heap: &'static mut UdonHeap) -> *mut InterpreterState {
    if let Some(interpreter) = InterpreterState::new(array, heap) {
        // println!("good");
        Box::into_raw(Box::new(interpreter))
    } else {
        println!("bad");
        std::ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn interpret(interpreter: &mut InterpreterState) -> bool {
    interpreter.interpret().is_some()
}

#[no_mangle]
pub extern "C" fn set_program_counter(interpreter: &mut InterpreterState, pc: u32) -> bool {
    interpreter.set_pc(pc as usize).is_some()
}

#[no_mangle]
pub extern "C" fn get_program_counter(interpreter: &InterpreterState) -> u32 {
    interpreter.pc as u32
}

#[no_mangle]
pub extern "C" fn dispose(interpreter: *mut InterpreterState) {
    unsafe {
        drop_in_place(interpreter);
        dealloc(interpreter as *mut u8, Layout::new::<InterpreterState>());
    }
}
