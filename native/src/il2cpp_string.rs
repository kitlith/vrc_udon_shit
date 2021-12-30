use std::fmt;

use crate::il2cpp_object::Il2CppObject;

#[repr(C)]
pub struct Il2CppString {
    _obj: Il2CppObject,
    _length: i32,
    _chars: u16,
}

impl Il2CppString {
    pub fn len(&self) -> usize {
        self._length as usize
    }
    pub fn as_slice(&self) -> &[u16] {
        unsafe { std::slice::from_raw_parts(&self._chars as *const u16, self.len()) }
    }
    pub fn to_string(&self) -> String {
        String::from_utf16_lossy(self.as_slice())
    }
}

impl fmt::Display for Il2CppString {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
