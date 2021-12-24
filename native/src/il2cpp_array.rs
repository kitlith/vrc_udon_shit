use crate::il2cpp_object::Il2CppObject;

use std::{slice, ops};

#[repr(C)]
pub struct Il2CppArray<T> {
    _obj: Il2CppObject,
    bounds: *const Il2CppArrayBounds,
    max_length: Il2cppArraySizeType,
    values: T,
}

type Il2cppArraySizeType = usize;
type Il2cppArrayLowerBoundType = u32;

#[repr(C)]
struct Il2CppArrayBounds {
    _length: Il2cppArraySizeType,
    _lower_bound: Il2cppArrayLowerBoundType,
}

impl<T> Il2CppArray<T> {
    pub unsafe fn as_slice(&self) -> &[T] {
        slice::from_raw_parts(self.start(), self.len())
    }
    // pub unsafe fn as_mut_slice(&mut self) -> &mut [T] {
    //     slice::from_raw_parts_mut(self.start_mut(), self.len())
    // }
    pub fn start(&self) -> *const T {
        &self.values as *const T
    }
    pub fn start_mut(&mut self) -> *mut T {
        &mut self.values as *mut T
    }
    pub fn len(&self) -> usize {
        self.max_length as usize
    }
}

impl<T> ops::Index<usize> for Il2CppArray<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        unsafe { &*self.start().add(index) }
    }
}

impl<T> ops::IndexMut<usize> for Il2CppArray<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe { &mut *self.start_mut().add(index) }
    }
}

#[test]
fn assert_sizes() {
    assert_eq!(32, std::mem::size_of::<Il2CppArray<()>>());
}
