// inline assembly is getting stabilized! but until it reaches beta, let's keep the feature flag.
#![allow(stable_features)]
#![feature(asm)]

mod delegate;
mod il2cpp_array;
mod il2cpp_object;
mod il2cpp_string;
mod span;
mod strongbox;
mod udon_types;

pub mod vm;
