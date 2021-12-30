use crate::delegate::Delegate;
use crate::il2cpp_array::Il2CppArray;
use crate::il2cpp_object::Il2CppObject;
use crate::il2cpp_string::Il2CppString;
use crate::span::Span;
use crate::strongbox::IStrongBox;

use libc::c_void;

// pub struct UdonProgram {
//     _obj: Il2CppObject,
//     pub instruction_set_identifier: *const Il2CppString,
//     pub byte_code: *const Il2CppArray<u8>,
//     pub heap: *mut UdonHeap,
//     pub entry_points: *const c_void,
//     pub symbol_table: *const c_void,
//     pub sync_metadata_table: *const c_void,
//     pub update_order: *const i32,
// }

#[repr(C)]
pub struct UdonHeap {
    _obj: Il2CppObject,
    heap: *mut Il2CppArray<*const IStrongBox>,
    _strong_box_of_type_cache: *const c_void,
    _strong_box_of_t_contained_type_cache: *const c_void,
}

impl UdonHeap {
    /* TODO: raw access into internal """heap""" */
    pub fn get_variable(&self, address: u32) -> &Il2CppObject {
        unsafe { ((*CALLBACKS).get_variable)(self, address) }
    }
    pub fn copy_variables(&mut self, src: u32, dst: u32) {
        unsafe { ((*CALLBACKS).copy_variables)(self, src, dst) }
    }
    pub fn get_value<T: Copy>(&self, address: u32) -> T {
        unsafe { *self.get_variable(address).unbox() }
    }
    pub fn get_object<T>(&self, address: u32) -> &T {
        unsafe { self.get_variable(address).cast() }
    }
    pub fn size(&self) -> usize {
        unsafe { (*self.heap).len() }
    }
    // pub fn set_raw(&mut self, address: u32, value: *const c_void) {
    //     unsafe { (*self.heap)[address as usize] = value as *const IStrongBox };
    // }
    // pub fn get_raw(&self, address: u32) -> *const c_void {
    //     unsafe { (*self.heap)[address as usize] as *const c_void }
    // }
}

pub struct IUdonWrapper {
    _obj: Il2CppObject,
}

impl IUdonWrapper {
    pub fn get_extern_function_delegate(
        &self,
        signature: &Il2CppString,
    ) -> &Delegate<UdonWrapperCallbackType> {
        unsafe { ((*CALLBACKS).get_extern_function_delegate)(self, signature) }
    }
    pub fn get_extern_function_parameter_count(&self, signature: &Il2CppString) -> i32 {
        unsafe { ((*CALLBACKS).get_extern_function_parameter_count)(self, signature) }
    }
}

pub struct UdonVMTimeSource {
    _obj: Il2CppObject,
    _timer: *const c_void,
    _current_time: i64,
}

pub type UdonWrapperCallbackType = extern "C" fn(&Il2CppObject, &mut UdonHeap, &Span<u32>);

pub struct CallbackTable {
    pub get_variable: extern "C" fn(heap: &UdonHeap, address: u32) -> &Il2CppObject,
    pub copy_variables: extern "C" fn(heap: &mut UdonHeap, src: u32, dst: u32),
    pub get_extern_function_delegate: extern "C" fn(
        wrapper: &IUdonWrapper,
        signature: &Il2CppString,
    )
        -> &'static Delegate<UdonWrapperCallbackType>,
    pub get_extern_function_parameter_count:
        extern "C" fn(wrapper: &IUdonWrapper, signature: &Il2CppString) -> i32,
}

/* WARNING: Must never invalidate on C# side */
static mut CALLBACKS: *const CallbackTable = 0 as *const CallbackTable;

#[no_mangle]
pub extern "C" fn set_function_pointers(callbacks: *const CallbackTable) {
    unsafe {
        CALLBACKS = callbacks;
    }
}

#[test]
fn assert_sizes() {
    assert_eq!(40, std::mem::size_of::<UdonHeap>());
}
