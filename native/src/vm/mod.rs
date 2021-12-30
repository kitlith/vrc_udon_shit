pub mod analysis;
pub mod dynarec;
pub mod emit;
mod interpreter;

use num_derive::FromPrimitive;

#[derive(FromPrimitive)]
pub enum OpCode {
    Nop = 0,
    Push = 1,
    Pop = 2,
    JumpIfFalse = 4,
    Jump = 5,
    Extern = 6,
    Annotation = 7,
    JumpIndirect = 8,
    Copy = 9,
    CachedExtern = 10,
}

// TODO: create a central state/context struct that the dynarec, interpreter, and vm all share?
// emit::Context was never meant to be the end-all be-all for what the dynarec needed,
// it was just the minimum necessary and was implemented while touching as little else as I could.

// TODO: move the main VM loop/the interface with C# here? then it can call out to the dynarec and the interpreter as necessary.
// i.e. the contents of dynarec.rs and a bit of interpreter.rs moves here, emit.rs becomes recompiler or dynarec.
// idk. i'm working on organization because I don't want to work on getting stuff to build correctly on my systems right now.
