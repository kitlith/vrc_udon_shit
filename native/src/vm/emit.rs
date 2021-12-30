use dynasmrt::{self, dynasm, DynamicLabel, DynasmApi, DynasmLabelApi};
use rustc_hash::FxHashMap;

use core::arch::asm;

use super::analysis::{ExternArgs, ReturnCode, StackOps};
use crate::il2cpp_object::Il2CppObject;
use crate::span::Span;
use crate::udon_types::UdonHeap;

extern "C" {
    // meant to be jumped to by jitted assembly!
    fn vm_code_return();
}

macro_rules! udon_dynasm {
    ($ops:ident $($t:tt)*) => {
        dynasm!($ops
            ; .arch x64
            // RBX, RBP, RDI, RSI, RSP, R12-R15 are callee-saved registers.
            // ditch RBP and RSP since those are the stack/frame pointers
            // RBX is reserved by llvm
            // remaining named registers are RDI, RSI
            // am gonna use R12-R15 first, because i don't want to think about special purpose registers that much.
            // NOTE: we could always stuff a bunch of stuff in the context struct and not have it around persistantly.
            ; .alias heap_ptr, r12
            ; .alias stack_base, r13
            ; .alias stack_count, r14
            ; .alias span_ptr, r15
            ; .alias context, rsi
            // return code/return bytecode pc
            ; .alias retval, rax
            ; .alias retval_32, eax
            ; .alias retval_8, al
            // arguments
            ; .alias arg1, rcx
            ; .alias arg1_32, ecx
            ; .alias arg2, rdx
            ; .alias arg2_32, edx
            ; .alias arg3, r8
            ; .alias arg3_32, r8d
            ; .alias arg4, r9
            ; .alias arg4_32, r9d
            $($t)*
        )
    }
}

pub struct Context {
    heap_ptr: *mut UdonHeap, // r12 / x19
    stack: Vec<u32>,
    stack_count: u64,
    span: crate::span::Span<u32>,
}

impl Context {
    pub fn new(heap: *mut UdonHeap) -> Self {
        Self {
            heap_ptr: heap,
            stack: Vec::with_capacity(0x1000),
            stack_count: 0,
            span: Span::<u32>::new(&[]),
        }
    }
    pub fn reserve_stack(&mut self, size: u64) -> *mut u32 {
        if size as usize > self.stack.len() {
            self.stack.resize(size as usize, 0u32);
        }
        self.stack.as_mut_ptr()
    }
    pub fn set_stack_count(&mut self, val: u64) {
        self.stack_count = val;
    }
    pub fn get_stack_count(&self) -> u64 {
        self.stack_count
    }
}

// NOTE: This has undefined behaviour! C++ exceptions are not supposed to unwind through rust functions.
// this seems to work anyway on windows? so why the fuck not.
#[no_mangle]
#[inline(never)] // we use a named label, cannot allow it to conflict.
#[allow(named_asm_labels)]
pub extern "C" fn call_vm_code(code_ptr: *const core::ffi::c_void, context: &mut Context) -> u64 {
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
            // heap pointer shouldn't change
            in("r12") context.heap_ptr,
            // stack pointer comes from vec, not controlled by the vm, but it does get updated in the vm
            inout("r13") context.stack.as_mut_ptr() => _,
            // we do want to keep track of what the vm does with this
            inout("r14") context.stack_count,
            in("r15") &context.span,
            in("rsi") context,
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
            // don't really care, but first argument/result already happens to be here
            inout("x0") code_ptr => ret,
            in("x19") context.heap_ptr,
            inout("x20") context.stack.as_mut_ptr() => _,
            inout("x21") context.stack_count,
            in("x22") &context.span,
            in("x23") context
            clobber_abi("system")
        );
    }
    // lower 32 bits should be bytecode PC, upper 32 bits should signal if there's an error/exception
    ret
}

pub fn emit(
    assembler: &mut dynasmrt::x64::Assembler,
    ops: &[StackOps],
    jump_table: &FxHashMap<u32, DynamicLabel>,
) {
    fn push_n(assembler: &mut dynasmrt::x64::Assembler, params: &[u32]) {
        for (offset, &value) in params.iter().enumerate() {
            udon_dynasm!(assembler
                ; mov DWORD [stack_base + stack_count * 0x4 + (offset * 4) as i32], DWORD value as _
            );
        }
        udon_dynasm!(assembler
            ; add stack_count, params.len() as _
        );
    }

    let mut min_stack: isize = 0;
    let mut max_stack: isize = 0;
    {
        let mut cur_stack: isize = 0;
        for op in ops {
            // NOTE: this match block is tied to the codegen down below.
            match op {
                StackOps::Push(params) => {
                    cur_stack += params.len() as isize;
                    max_stack = std::cmp::max(max_stack, cur_stack);
                }
                StackOps::Pop(count) => {
                    cur_stack -= *count as isize;
                }
                StackOps::Extern { args, .. } => {
                    // NOTE: eventually complete externs won't push to the stack.
                    if let ExternArgs::Complete(args) = args {
                        cur_stack += args.len() as isize;
                        max_stack = std::cmp::max(max_stack, cur_stack);
                    }
                    cur_stack -= args.len() as isize;
                }
                StackOps::JumpIfFalse { arg: Some(_), .. } => continue,
                StackOps::JumpIfFalse { arg: None, .. } => {
                    cur_stack -= 1;
                }
                StackOps::CopyComplete { .. } => continue,
                StackOps::CopyIncomplete => {
                    cur_stack -= 2;
                }
                StackOps::Jump(_) | StackOps::JumpIndirect(_) | StackOps::Return(_) => continue,
            }

            min_stack = std::cmp::min(min_stack, cur_stack);
        }
    }

    // i.e.: there are incomplete operations
    if min_stack < 0 {
        udon_dynasm!(assembler
            ; cmp stack_count, (-min_stack) as u32 as _
            ; jge >skip_jmp
            // if count < minimal stack removal
            // this is not a happy path, we don't care about speed here, VM is about to halt.
            // TODO: embed block PC? or should we just have trampoline infer from which block just returned?
            ; mov retval, QWORD ReturnCode::RequestInterpreter(0).encode() as _
            ; mov rcx, QWORD vm_code_return as _
            // return RequestInterpreter
            ; jmp rcx
            // else continue
            ; skip_jmp:
        );
    }

    // this will almost always be true...
    if max_stack > 0 {
        /* Extend stack */
        udon_dynasm!(assembler
            ; mov arg1, context
            ; mov arg2, max_stack as _
            ; add arg2, stack_count
            // Context::reserve_stack(context, max_stack+stack_count)
            ; mov rax, QWORD Context::reserve_stack as _
            ; call rax
            ; mov stack_base, retval
        );
    }

    // NOTE: we could probably choose to only update the stack at the end of the block
    // and just keep track of the current stack offset instead of updating it with add/sub
    // (ghidra shows the function as if we had chosen to do that, which is what reminded me.)

    for op in ops {
        match op {
            StackOps::Push(params) => {
                push_n(assembler, &params);
            }
            StackOps::Pop(count) => {
                udon_dynasm!(assembler
                    ; sub stack_count, *count as _
                );
            }
            StackOps::Extern {
                callback,
                target,
                heap_slot: _,
                args,
            } => {
                let count = match &args {
                    ExternArgs::Complete(params) => {
                        // TODO: allocate a complete slice somewhere that we can embed a pointer to
                        push_n(assembler, params);
                        params.len()
                    }
                    ExternArgs::Incomplete(count) => {
                        /* Arguments are already pushed to the stack */
                        *count
                    }
                };
                let addr = *target as *const Il2CppObject;
                udon_dynasm!(assembler
                    ; sub stack_count, count as _

                    // TODO: is this necessary? potential workaround for some stack underflow bug.
                    ; mov arg1, context
                    ; mov arg2, stack_count
                    ; mov rax, QWORD Context::set_stack_count as _
                    ; call rax

                    ; lea arg3, [stack_base + stack_count * 4]
                    /* Construct Span<u32> pointing to stack */
                    ; mov QWORD [span_ptr + 0x00], 0
                    ; mov QWORD [span_ptr + 0x08], arg3
                    ; mov DWORD [span_ptr + 0x10], count as _

                    ; mov arg1, QWORD addr as _
                    ; mov arg2, heap_ptr
                    ; mov arg3, span_ptr
                    /* Invoke extern */
                    // Extern(target, heap, arguments_span)
                    ; mov rax, QWORD *callback as _
                    ; call rax

                    // TODO: is this necessary? potential workaround for some stack underflow bug.
                    ; mov arg1, context
                    ; mov rax, QWORD Context::get_stack_count as _
                    ; call rax

                    ; mov stack_count, retval

                    ; mov arg1, context
                    ; xor arg2, arg2 // arg2 = 0
                    ; mov rax, QWORD Context::reserve_stack as _
                    ; call rax
                );
            }
            StackOps::JumpIfFalse { destination, arg } => {
                /* Load bool flag from heap */
                if let Some(address) = arg {
                    udon_dynasm!(assembler
                        ; mov arg2, *address as _
                    );
                } else {
                    udon_dynasm!(assembler
                        ; sub stack_count, 1
                        ; mov arg2_32, DWORD [stack_base + stack_count * 4]
                    );
                }
                let jump_target = jump_table[destination];
                udon_dynasm!(assembler
                    ; mov arg1, heap_ptr
                    ; mov rax, QWORD UdonHeap::get_value::<bool> as _
                    // UdonHeap::get_value(heap, slot_address)
                    ; call rax

                    ; test retval_8, retval_8 // set ZF
                    ; jnz >skip_jmp
                    ; jmp => jump_target
                    ; skip_jmp:
                );
            }
            StackOps::CopyComplete { src, dst } => {
                udon_dynasm!(assembler
                    ; mov arg1, heap_ptr // &heap
                    ; mov arg2_32, DWORD *src as _
                    ; mov arg3_32, DWORD *dst as _
                    ; mov rax, QWORD UdonHeap::copy_variables as _
                    // UdonHeap::copy_variables(heap, src, dst)
                    ; call rax
                );
            }
            StackOps::CopyIncomplete => {
                udon_dynasm!(assembler
                    ; sub stack_count, 2
                    ; mov arg1, heap_ptr
                    ; mov arg2_32, DWORD [stack_base + stack_count * 4 + 0x0]
                    ; mov arg3_32, DWORD [stack_base + stack_count * 4 + 0x4]
                    ; mov rax, QWORD UdonHeap::copy_variables as _
                    // UdonHeap::copy_variables(heap, src, dst)
                    ; call rax
                );
            }
            StackOps::Jump(destination) => {
                let destination = *destination as u32;
                let jump_target = jump_table[&destination];
                udon_dynasm!(assembler
                    ; jmp =>jump_target
                );
            }
            StackOps::JumpIndirect(address) => {
                udon_dynasm!(assembler
                    ; mov arg1, heap_ptr
                    ; mov arg2_32, DWORD *address as _
                    ; mov rax, QWORD UdonHeap::get_value::<u32> as _
                    ; call rax

                    // zero upper half of return, just in case.
                    ; mov retval_32, retval_32

                    // TODO: attempt to query for existing block before returning
                    ; mov rcx, QWORD vm_code_return as _
                    // return indirect_destination
                    ; jmp rcx
                );
            }
            StackOps::Return(reason) => {
                udon_dynasm!(assembler
                    ; mov retval, QWORD reason.encode() as _
                    ; mov rcx, QWORD vm_code_return as _
                    // return reason
                    ; jmp rcx
                );
            }
        }
    }
}
