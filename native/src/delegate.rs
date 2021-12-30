use libc::c_void;

use crate::il2cpp_object::Il2CppObject;

#[repr(C)]
pub struct Delegate<T> {
    _obj: Il2CppObject,
    pub method_ptr: T,
    pub invoke_impl: *const c_void,
    pub m_target: &'static Il2CppObject,
}
