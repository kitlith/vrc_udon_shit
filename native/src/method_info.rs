use std::os::raw::{c_void, c_char};
use crate::il2cpp_class::*;

pub struct MethodInfo {
    pub method_ptr: *const c_void,
    pub invoker_method: *const c_void,
    pub name: *const c_char,
    pub class: *mut Il2CppClass,
    pub return_type: *const Il2CppType,
}
