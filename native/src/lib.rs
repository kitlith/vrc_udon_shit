// inline assembly is getting stabilized! but until it reaches beta, let's keep the feature flag.
#![allow(stable_features)]
#![feature(asm)]

mod delegate;
pub mod method_info;
pub mod il2cpp_array;
pub mod il2cpp_object;
pub mod il2cpp_string;
pub mod il2cpp_class;
mod span;
mod strongbox;
pub mod udon_types;

pub mod vm;
