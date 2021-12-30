use dynasmrt::DynasmLabelApi;
use dynasmrt::x64::Assembler;
use native::vm::analysis::{ExternArgs, ReturnCode, StackOps};
use native::vm::emit::emit;
use rustc_hash::FxHashMap;

fn main() {
    let mut ops = vec![
        StackOps::Push(vec![10, 11]),
        StackOps::Pop(2),
        StackOps::Extern {
            callback: unsafe { std::mem::transmute(0xDEADBEEF00000000u64) },
            target: unsafe { std::mem::transmute(0xCAFEBABE00000000u64) },
            heap_slot: 12,
            args: ExternArgs::Complete(vec![4, 8, 15, 16, 23, 42]),
        },
        StackOps::Extern {
            callback: unsafe { std::mem::transmute(0xDEADBEEF00000001u64) },
            target: unsafe { std::mem::transmute(0xCAFEBABE00000001u64) },
            heap_slot: 13,
            args: ExternArgs::Incomplete(2),
        }, // -2
        StackOps::JumpIfFalse {
            destination: 0xDEAD0001,
            arg: Some(14),
        },
        StackOps::JumpIfFalse {
            destination: 0xDEAD0002,
            arg: None,
        }, // -3
        StackOps::CopyComplete { src: 15, dst: 16 },
        StackOps::CopyIncomplete, // -5
        StackOps::Jump(0xDEAD0003),
    ];

    let mut assembler = Assembler::new().unwrap();
    let mut labels = FxHashMap::default();
    labels.insert(0xDEAD0001, assembler.new_dynamic_label());
    labels.insert(0xDEAD0002, assembler.new_dynamic_label());
    labels.insert(0xDEAD0003, assembler.new_dynamic_label());

    assembler.dynamic_label(labels[&0xDEAD0001]);
    emit(&mut assembler, &ops, &labels);

    assembler.dynamic_label(labels[&0xDEAD0002]);
    *ops.last_mut().unwrap() = StackOps::Return(ReturnCode::RequestInterpreter(0xDEAD0004));
    emit(&mut assembler, &ops, &labels);

    assembler.dynamic_label(labels[&0xDEAD0003]);
    *ops.last_mut().unwrap() = StackOps::JumpIndirect(0xBEEF);
    emit(&mut assembler, &ops, &labels);

    let result = assembler.finalize().unwrap();

    std::fs::write("test.bin", result.as_ref()).unwrap();
}
