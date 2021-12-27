use dynasmrt::{self, dynasm, DynasmApi, DynasmLabelApi};

use crate::il2cpp_object::Il2CppObject;
use crate::recompiler::{ExternArgs, StackOps};
use crate::udon_types::UdonHeap;

pub fn emit(ops: &[StackOps]) {
    let mut assembler = dynasmrt::x64::Assembler::new().unwrap();

    dynasm!(assembler
        /* Push heap to rsi and keep it there */
        ; mov rsi, rdi
    );

    fn push_n(assembler: &mut dynasmrt::x64::Assembler, params: &[u32]) {
        /* Extend stack */
        dynasm!(assembler
            ; sub rsp, (params.len() * 0x4) as i32
        );
        for (offset, &value) in params.iter().enumerate() {
            dynasm!(assembler
                ; mov DWORD [rsp + (offset * 0x4) as i32], value as i32
            );
        }
    }

    for op in ops {
        match op {
            StackOps::Push(params) => {
                push_n(&mut assembler, &params);
            }
            StackOps::Pop(count) => {
                dynasm!(assembler
                    ; add rsp, (count * 0x4) as i32
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
                        push_n(&mut assembler, params);
                        params.len()
                    }
                    ExternArgs::Incomplete(count) => {
                        /* Arguments are already pushed to the stack */
                        *count
                    }
                };
                let addr = *target as *const Il2CppObject;
                dynasm!(assembler
                    /* Construct Span<u32> pointing to stack top */
                    ; mov rdx, rsp
                    ; sub rsp, BYTE 0x18
                    ; mov QWORD [rsp + 0x00], 0
                    ; mov QWORD [rsp + 0x08], rdx
                    ; mov DWORD [rsp + 0x10], count as _
                    
                    ; mov rdx, rsp // &span
                    /* Heap already in position */
                    ; mov rdi, addr as _ // &target
                    /* Invoke extern */
                    ; call *callback as _
                    ; add rsp, BYTE 0x18 + (count * 0x4) as i8
                );
            }
            StackOps::JumpIfFalse {
                destination,
                arg,
            } => {
                /* Load bool flag from heap */
                if let Some(address) = arg {
                    dynasm!(assembler
                        ; mov rdx, *address as _
                    );
                } else {
                    dynasm!(assembler
                        ; pop rdx
                    );
                }
                dynasm!(assembler
                    ; mov rdi, rsi // &heap
                    ; call UdonHeap::get_value::<bool> as _
                    /* TODO: fix jump destination */
                    ; jz *destination as _
                    ; mov rsi, rdi
                );
                unimplemented!();
            }
            StackOps::CopyComplete { src, dst } => {
                dynasm!(assembler
                    ; mov rdi, rsi // &heap
                    ; mov esi, *src as _
                    ; mov edx, *dst as _
                    ; call UdonHeap::copy_variables as _
                    ; mov rsi, rdi
                );
            }
            StackOps::CopyIncomplete => {
                dynasm!(assembler
                    ; mov rdi, rsi // &heap
                    ; mov esi, [rsp + 0x8] // src
                    ; mov edx, [rsp + 0x0] // dst
                    ; call UdonHeap::copy_variables as _
                    ; mov rsi, rdi
                );
            }
            StackOps::Jump(destination) => {
                /* TODO */
                unimplemented!();
            }
            StackOps::JumpIndirect(address) => {
                /* TODO */
                unimplemented!();
            }
            _ => {
                panic!("Unknown StackOps");
            }
        }
        // dynasm!(assembler)
    }

    let block = assembler.finalize().unwrap();
}
