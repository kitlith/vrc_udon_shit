use dynasmrt::{self, dynasm, DynasmApi, DynasmLabelApi};

use crate::il2cpp_object::Il2CppObject;
use crate::recompiler::{ExternArgs, StackOps, ReturnCode};
use crate::udon_types::UdonHeap;

struct Context {
    stack: Vec<u32>,
    // ...?
}

impl Context {
    fn reserve_stack(&mut self, size: u64) -> *mut u32 {
        self.stack.resize(size as usize, 0u32);
        self.stack.as_mut_ptr()
    }
}

extern "C" {
    // meant to be jumped to by jitted assembly!
    fn vm_code_return();
}

macro_rules! udon_dynasm {
    ($ops:ident $($t:tt)*) => {
        dynasm!($ops
            ; .arch x64
            // RBX, RBP, RDI, RSI, RSP, R12-R15 are callee-saved registers.
            // ditch RBP and RSP since those mess with the stack/frame pointers
            // remaining named registers are RBX, RDI, RSI
            // am gonna use R12-R15 first, because i don't want to think about special purpose registers.
            // let's leave rdi/rsi for rep movs copying.
            // NOTE: we could always stuff a bunch of stuff in the context struct and not have it around persistantly.
            ; .alias heap_ptr, r12
            ; .alias stack_base, r13
            ; .alias stack_count, r14
            ; .alias span_ptr, r15
            ; .alias context, rbx
            // return code/return bytecode pc
            ; .alias retval, rax
            // arguments
            ; .alias arg1, rcx
            ; .alias arg2, rdx
            ; .alias arg2_32, edx
            ; .alias arg3, r8
            ; .alias arg4, r9
            $($t)*
        )
    }
}

pub fn emit(ops: &[StackOps]) {
    let mut assembler = dynasmrt::x64::Assembler::new().unwrap();

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
                StackOps::CopyComplete {..} => continue,
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

    for op in ops {
        match op {
            StackOps::Push(params) => {
                push_n(&mut assembler, &params);
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
                args
            } => {
                let count = match &args {
                    ExternArgs::Complete(params) => {
                        // TODO: allocate a complete slice somewhere that we can embed a pointer to
                        push_n(&mut assembler, params);
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
                );
            }
            StackOps::JumpIfFalse {
                destination,
                arg,
            } => {
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
                udon_dynasm!(assembler
                    ; mov arg1, heap_ptr
                    ; mov rax, QWORD UdonHeap::get_value::<bool> as _
                    // UdonHeap::get_value(heap, slot_address)
                    ; call rax

                    ; test retval, retval // set ZF
                    ; jnz >skip_jmp
                    // retval houses destination address
                    ; mov retval, DWORD *destination as _
                    // TODO: save function pointer offset for future modification
                    ; mov rcx, QWORD vm_code_return as _
                    // return destination
                    ; jmp rcx
                    ; skip_jmp:
                );
            }
            StackOps::CopyComplete { src, dst } => {
                udon_dynasm!(assembler
                    ; mov arg1, heap_ptr // &heap
                    ; mov arg2, DWORD *src as _
                    ; mov arg3, DWORD *dst as _
                    ; mov rax, QWORD UdonHeap::copy_variables as _
                    // UdonHeap::copy_variables(heap, src, dst)
                    ; call rax
                );
            }
            StackOps::CopyIncomplete => {
                udon_dynasm!(assembler
                    ; sub stack_count, 2
                    ; mov arg1, heap_ptr
                    ; mov arg2, [stack_base + 0x0]
                    ; mov arg3, [stack_base + 0x4]
                    ; mov rax, QWORD UdonHeap::copy_variables as _
                    // UdonHeap::copy_variables(heap, src, dst)
                    ; call rax
                );
            }
            StackOps::Jump(destination) => {
                udon_dynasm!(assembler
                    ; mov retval, DWORD *destination as _
                    // TODO: save function pointer offset for future modification
                    ; mov rcx, QWORD vm_code_return as _
                    // return destination
                    ; jmp rcx
                );
            }
            StackOps::JumpIndirect(address) => {
                udon_dynasm!(assembler
                    ; mov arg1, heap_ptr
                    ; mov arg2, DWORD *address as _
                    ; mov rax, QWORD UdonHeap::get_value::<u32> as _
                    ; call rax

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
            // _ => {
            //     panic!("Unknown StackOps");
            // }
        }
        // dynasm!(assembler)
    }

    let block = assembler.finalize().unwrap();
}
