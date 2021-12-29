use native::vm::emit::emit;
use native::vm::analysis::{ExternArgs, StackOps, ReturnCode};

fn main() {

    let mut ops = vec![
        StackOps::Push(vec![10, 11]),
        StackOps::Pop(2),
        StackOps::Extern {
            callback: unsafe { std::mem::transmute(0xDEADBEEF00000000u64) },
            target: unsafe { std::mem::transmute(0xCAFEBABE00000000u64) },
            heap_slot: 12,
            args: ExternArgs::Complete(vec![4, 8, 15, 16, 23, 42])
        },
        StackOps::Extern {
            callback: unsafe { std::mem::transmute(0xDEADBEEF00000001u64) },
            target: unsafe { std::mem::transmute(0xCAFEBABE00000001u64) },
            heap_slot: 13,
            args: ExternArgs::Incomplete(2)
        }, // -2
        StackOps::JumpIfFalse { destination: 0xDEAD0001, arg: Some(14)},
        StackOps::JumpIfFalse { destination: 0xDEAD0002, arg: None}, // -3
        StackOps::CopyComplete { src: 15, dst: 16 },
        StackOps::CopyIncomplete, // -5
        StackOps::Jump(0xDEAD0003),
    ];

    let mut result = Vec::<u8>::new();

    result.extend_from_slice(emit(&ops).as_ref());

    *ops.last_mut().unwrap() = StackOps::Return(ReturnCode::RequestInterpreter(0xDEAD0004));
    result.extend_from_slice(emit(&ops).as_ref());

    *ops.last_mut().unwrap() = StackOps::JumpIndirect(0xBEEF);
    result.extend_from_slice(emit(&ops).as_ref());

    std::fs::write("test.bin", &result).unwrap();
}