use libc::c_void;
use std::mem;

use crate::il2cpp_class::Il2CppClass;

#[repr(C)]
pub struct Il2CppObject {
    pub klass: *const Il2CppClass,
    monitor: *const c_void,
}

impl Il2CppObject {
    pub unsafe fn unbox<T>(&self) -> &T {
        let start = self as *const Self as usize + mem::size_of::<Il2CppObject>();
        (start as *const T).as_ref().unwrap()
    }
    pub unsafe fn cast<T>(&self) -> &T {
        mem::transmute(self)
    }
}

impl Default for Il2CppObject {
    fn default() -> Self {
        Self {
            klass: 0 as *const Il2CppClass,
            monitor: 0 as *const c_void,
        }
    }
}

#[test]
fn assert_sizes() {
    assert_eq!(16, mem::size_of::<Il2CppObject>());
}
