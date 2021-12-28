use crate::il2cpp_object::Il2CppObject;

#[repr(C)]
pub struct IStrongBox {
    _obj: Il2CppObject,
}

// impl IStrongBox {
//     pub fn get_value<T>(&self) -> T {
//         unimplemented!()
//     }
// }

// pub struct StrongBox<T> {
//     _obj: Il2CppObject,
//     pub value: T
// }
