// inline assembly is getting stabilized! but until it reaches beta, let's keep the feature flag.
#![allow(stable_features)]
#![feature(asm)]

mod il2cpp_array;
mod il2cpp_object;
mod il2cpp_method;
mod il2cpp_string;
mod delegate;
mod strongbox;
mod span;
mod udon_types;
mod interpreter;
mod recompiler;
mod emit;