use std::{
    collections::HashMap,
    ffi::{c_void, CStr, CString},
    intrinsics::transmute,
    ptr::{null, null_mut},
    slice, thread,
    time::Duration,
};

use windows_dll::*;

use native::il2cpp_class::*;
use native::il2cpp_object::*;
use native::il2cpp_string::*;
use native::udon_types::*;
use native::vm::interpreter::*;
use native::{il2cpp_array::*, method_info::MethodInfo};

struct Il2CppDomain;
struct Il2CppAssembly;
// struct Il2CppImage;
// struct Il2CppClass;
#[repr(C)]
struct Il2CppException {
    _obj: Il2CppObject,
    class_name: *mut Il2CppString,
    message: *mut Il2CppString,
    _data: *mut Il2CppObject,
    inner_ex: *mut Il2CppException,
    _helpURL: *mut Il2CppString,
    trace_ips: *mut Il2CppArray<*mut MethodInfo>,
    stack_trace: *mut Il2CppString,
    remote_stack_trace: *mut Il2CppString,
    remote_stack_index: i32,
    _dynamicMethods: *mut Il2CppObject,
    hresult: i32,
    source: *mut Il2CppString,
    safeSerializationManager: *mut Il2CppObject,
    captured_traces: *mut Il2CppArray<*mut Il2CppObject>,
    native_trace_ips: *mut Il2CppArray<*mut Il2CppObject>,
}
// struct Il2CppObject;
// struct Il2CppString;

struct FieldInfo;

type CallbackType = extern "C" fn(message: *const i8);

#[dll("UnityPlayer.dll")]
extern "system" {
    fn UnityMain(a: *const c_void, b: *const c_void, c: *const u32, d: i32) -> i32;
}

#[dll("Kernel32.dll")]
extern "system" {
    fn GetLastError() -> i32;
}

#[dll("GameAssembly.dll")]
extern "system" {
    fn il2cpp_init(domain_name: *const u8) -> i32;
    fn il2cpp_shutdown();
    fn il2cpp_set_data_dir(data_dir: *const u8);

    // logging
    fn il2cpp_register_log_callback(method: CallbackType);

    // domain
    fn il2cpp_domain_get() -> *const Il2CppDomain;
    fn il2cpp_domain_get_assemblies(
        domain: *const Il2CppDomain,
        size: &mut usize,
    ) -> *const *const Il2CppAssembly;

    // assembly
    fn il2cpp_assembly_get_image(assembly: *const Il2CppAssembly) -> *const Il2CppImage;

    // image
    fn il2cpp_image_get_assembly(image: *const Il2CppImage) -> *const Il2CppAssembly;
    fn il2cpp_image_get_name(image: *const Il2CppImage) -> *const i8;
    fn il2cpp_image_get_filename(image: *const Il2CppImage) -> *const i8;

    // class
    fn il2cpp_class_from_name(
        image: *const Il2CppImage,
        namespace: *const u8,
        name: *const u8,
    ) -> *const Il2CppClass;
    fn il2cpp_class_get_field_from_name(
        class: *const Il2CppClass,
        name: *const u8,
    ) -> *const FieldInfo;
    fn il2cpp_class_get_method_from_name(
        class: *const Il2CppClass,
        name: *const u8,
        args: i32,
    ) -> *const MethodInfo;
    fn il2cpp_class_from_type(type_: *const Il2CppType) -> *const Il2CppClass;
    fn il2cpp_class_get_type(klass: *const Il2CppClass) -> *const Il2CppType;

    // field
    fn il2cpp_field_static_get_value(field: *const FieldInfo, value: *mut c_void);

    // runtime
    fn il2cpp_runtime_invoke(
        method: *const MethodInfo,
        obj: *mut c_void,
        args: *const *mut c_void,
        exc: *const *mut Il2CppException,
    ) -> *mut Il2CppObject;
    fn il2cpp_runtime_class_init(klass: *const Il2CppClass);

    // object
    fn il2cpp_object_get_virtual_method(
        obj: *mut Il2CppObject,
        method: *const MethodInfo,
    ) -> *const MethodInfo;
    fn il2cpp_object_new(class: *const Il2CppClass) -> *mut Il2CppObject;
    fn il2cpp_object_unbox(obj: *const Il2CppObject) -> *mut c_void;

    // string
    fn il2cpp_string_new(str: *const u8) -> *mut Il2CppString;

    // array
    fn il2cpp_array_new(element_type_info: *const Il2CppClass, length: usize) -> *mut c_void;

    // exception
    fn il2cpp_format_exception(exc: *const Il2CppException, buffer: *mut u8, buffer_size: i32);
    fn il2cpp_format_stack_trace(exc: *const Il2CppException, buffer: *mut u8, buffer_size: i32);
}

fn main() {
    unsafe {
        println!("{:?}", il2cpp_init::load());
        println!("{:?}", GetLastError());
        if !UnityMain::exists() {
            println!("UnityPlayer.dll not found");
            println!("{:?}", UnityMain::load());
            println!("{:?}", GetLastError::exists());
            println!("{:?}", GetLastError());
            return;
        }

        let ret = UnityMain(null(), null(), null(), 0);

        println!("ret: {}", ret);
    }

    return;
    if !il2cpp_init::exists() {
        println!("GameAssembly not found");
        return;
    }

    // thread::sleep(Duration::from_secs(3));

    /* Fun! */
    unsafe {
        il2cpp_set_data_dir(
            "C:\\Program Files (x86)\\Steam\\steamapps\\common\\VRChat\\VRChat_Data\\il2cpp_data\0"
                .as_ptr(),
        );

        // if il2cpp_init("Shit\0".as_ptr()) == 0 {
        //     panic!("Failed to initialize");
        // }

        extern "C" fn log_callback(message: *const i8) {
            println!("{}", unsafe { CStr::from_ptr(message) }.to_str().unwrap());
        }
        il2cpp_register_log_callback(log_callback);

        let domain = il2cpp_domain_get();

        let mut size = 0;
        let assemblies = il2cpp_domain_get_assemblies(domain, &mut size);

        let image_map = slice::from_raw_parts(assemblies, size)
            .iter()
            .map(|&assembly| {
                let image = il2cpp_assembly_get_image(assembly);
                let name = il2cpp_image_get_name(image);
                (CStr::from_ptr(name).to_str().unwrap(), image)
            })
            .collect::<HashMap<&str, *const Il2CppImage>>();

        let mut exc: *mut Il2CppException = std::ptr::null_mut();

        let blacklist_class = il2cpp_class_from_name(
            image_map["VRC.Udon.Security.dll"],
            "VRC.Udon.Security\0".as_ptr(),
            "UnityEngineObjectSecurityBlacklist\0".as_ptr(),
        );
        il2cpp_runtime_class_init(blacklist_class);
        let blacklist_ctor =
            il2cpp_class_get_method_from_name(blacklist_class, ".ctor\0".as_ptr(), 0);

        let blacklist = il2cpp_object_new(blacklist_class);
        il2cpp_runtime_invoke(
            blacklist_ctor,
            blacklist as *mut c_void,
            std::ptr::null_mut(),
            &mut exc,
        );
        if exc != std::ptr::null_mut() {
            println!("Failed to invoke blacklist ctor");
            print_exception(exc);
            return;
        }

        let wrapper_factory_class = il2cpp_class_from_name(
            image_map["VRC.Udon.Wrapper.dll"],
            "VRC.Udon.Wrapper\0".as_ptr(),
            "UdonDefaultWrapperFactory\0".as_ptr(),
        );
        il2cpp_runtime_class_init(wrapper_factory_class);
        let wrapper_factory_ctor =
            il2cpp_class_get_method_from_name(wrapper_factory_class, ".ctor\0".as_ptr(), 1);

        let wrapper_factory = il2cpp_object_new(wrapper_factory_class);
        let args = [blacklist as *mut c_void];
        il2cpp_runtime_invoke(
            wrapper_factory_ctor,
            wrapper_factory as *mut c_void,
            args.as_ptr(),
            &mut exc as _,
        );
        if exc != std::ptr::null_mut() {
            println!("Failed to invoke wrapper factory ctor");
            print_exception(exc);
            return;
        }

        let get_wrapper =
            il2cpp_class_get_method_from_name(wrapper_factory_class, "GetWrapper\0".as_ptr(), 0);
        let wrapper = il2cpp_runtime_invoke(
            get_wrapper,
            wrapper_factory as *mut c_void,
            std::ptr::null_mut(),
            &mut exc as _,
        );
        if exc != std::ptr::null_mut() {
            println!("Failed to invoke get_wrapper");
            print_exception(exc);
            return;
        }

        set_wrapper_and_time_source(wrapper as *const IUdonWrapper, std::ptr::null());

        let wrapper_class = il2cpp_class_from_name(
            image_map["VRC.Udon.Wrapper.dll"],
            "VRC.Udon.Wrapper\0".as_ptr(),
            "UdonWrapper\0".as_ptr(),
        );
        il2cpp_runtime_class_init(wrapper_class);
        let get_extern_function_delegate = il2cpp_class_get_method_from_name(
            wrapper_class,
            "GetExternFunctionDelegate\0".as_ptr(),
            1,
        );
        let get_extern_function_parameter_count = il2cpp_class_get_method_from_name(
            wrapper_class,
            "GetExternFunctionParameterCount\0".as_ptr(),
            1,
        );

        let udon_heap_class = il2cpp_class_from_name(
            image_map["VRC.Udon.Common.dll"],
            "VRC.Udon.Common\0".as_ptr(),
            "UdonHeap\0".as_ptr(),
        );
        il2cpp_runtime_class_init(udon_heap_class);
        let get_variable =
            il2cpp_class_get_method_from_name(udon_heap_class, "GetHeapVariable\0".as_ptr(), 1);
        let copy_variable =
            il2cpp_class_get_method_from_name(udon_heap_class, "CopyHeapVariable\0".as_ptr(), 2);

        let callbacks = CallbackTable {
            get_variable: std::mem::transmute((*get_variable).method_ptr),
            copy_variables: std::mem::transmute((*copy_variable).method_ptr),
            get_extern_function_delegate: std::mem::transmute(
                (*get_extern_function_delegate).method_ptr,
            ),
            get_extern_function_parameter_count: std::mem::transmute(
                (*get_extern_function_parameter_count).method_ptr,
            ),
        };
        set_function_pointers(&callbacks as _);

        let serialized_udon_program_asset_class = il2cpp_class_from_name(
            image_map["VRC.Udon.dll"],
            "VRC.Udon.ProgramSources\0".as_ptr(),
            "SerializedUdonProgramAsset\0".as_ptr(),
        );
        il2cpp_runtime_class_init(serialized_udon_program_asset_class);
        let serialized_udon_program_asset_ctor = il2cpp_class_get_method_from_name(
            serialized_udon_program_asset_class,
            ".ctor\0".as_ptr(),
            0,
        );
        let retrieve_program = il2cpp_class_get_method_from_name(
            serialized_udon_program_asset_class,
            "RetrieveProgram\0".as_ptr(),
            0,
        );
        let serialized_udon_program_asset = il2cpp_object_new(serialized_udon_program_asset_class);
        // il2cpp_runtime_invoke(
        //     serialized_udon_program_asset_ctor,
        //     serialized_udon_program_asset as *mut c_void,
        //     std::ptr::null_mut(),
        //     &mut exc as _,
        // );
        // if exc != std::ptr::null_mut() {
        //     println!("Failed to invoke serialized_udon_program_asset ctor");
        //     print_exception(exc);
        //     return;
        // }

        let list_class = il2cpp_class_from_name(
            image_map["mscorlib.dll"],
            "System.Collections.Generic\0".as_ptr(),
            "List`1\0".as_ptr(),
        );
        let unityengine_object_class = il2cpp_class_from_name(
            image_map["UnityEngine.CoreModule.dll"],
            "UnityEngine\0".as_ptr(),
            "Object\0".as_ptr(),
        );
        il2cpp_runtime_class_init(unityengine_object_class);
        let type_class = il2cpp_class_from_name(
            image_map["mscorlib.dll"],
            "System\0".as_ptr(),
            "Type\0".as_ptr(),
        );
        il2cpp_runtime_class_init(type_class);
        let runtime_type_class = il2cpp_class_from_name(
            image_map["mscorlib.dll"],
            "System\0".as_ptr(),
            "RuntimeType\0".as_ptr(),
        );
        il2cpp_runtime_class_init(runtime_type_class);
        let internal_from_handle =
            il2cpp_class_get_method_from_name(type_class, "internal_from_handle\0".as_ptr(), 1);
        let make_generic_type =
            il2cpp_class_get_method_from_name(runtime_type_class, "MakeGenericType\0".as_ptr(), 2);

        // let list_type = &(*list_class).byval_arg as *const Il2CppType;
        let list_type = il2cpp_class_get_type(list_class);
        let list_type_ptr = &list_type as *const *const Il2CppType;
        let list_type = il2cpp_runtime_invoke(
            internal_from_handle,
            null_mut(),
            [list_type_ptr as _].as_ptr(),
            &mut exc as _,
        );
        if exc != std::ptr::null_mut() {
            println!("Failed to invoke internal_from_handle for list_class");
            print_exception(exc);
            return;
        }
        println!(
            "list_type: {} {}",
            CStr::from_ptr(list_type.read().klass.read().namespaze)
                .to_str()
                .unwrap(),
            CStr::from_ptr(list_type.read().klass.read().name)
                .to_str()
                .unwrap(),
        );

        // let unityengine_object_type = &(*unityengine_object_class).byval_arg as *const Il2CppType;
        let unityengine_object_type = il2cpp_class_get_type(unityengine_object_class);
        let unityengine_object_type_ptr = &unityengine_object_type as *const *const Il2CppType;
        let unityengine_object_type = il2cpp_runtime_invoke(
            internal_from_handle,
            null_mut(),
            [unityengine_object_type_ptr as _].as_ptr(),
            &mut exc as _,
        );
        if exc != std::ptr::null_mut() {
            println!("Failed to invoke internal_from_handle for unityengine_object_class");
            print_exception(exc);
            return;
        }
        println!(
            "unityengine_object_type: {} {}",
            CStr::from_ptr(unityengine_object_type.read().klass.read().namespaze)
                .to_str()
                .unwrap(),
            CStr::from_ptr(unityengine_object_type.read().klass.read().name)
                .to_str()
                .unwrap(),
        );

        let array = il2cpp_array_new(type_class, 1);
        let type_array = &mut (array as *mut Il2CppArray<*const SystemType>).read();
        println!(
            "type_array: {}.{}",
            CStr::from_ptr(type_array.obj.klass.read().namespaze)
                .to_str()
                .unwrap(),
            CStr::from_ptr(type_array.obj.klass.read().name)
                .to_str()
                .unwrap(),
        );
        type_array[0] = unityengine_object_type as _;
        // let object_list_type = il2cpp_runtime_invoke(
        //     // il2cpp_object_get_virtual_method(list_type, make_generic_type),
        //     make_generic_type,
        //     null_mut(),
        //     [list_type as *mut _, array as *mut _].as_ptr(),
        //     &mut exc as _,
        // );
        // if exc != std::ptr::null_mut() {
        //     println!("Failed to invoke make_generic_type");
        //     print_exception(exc);
        //     panic!();
        // }
        type MakeType = fn(
            *mut SystemType,
            *const Il2CppArray<*const SystemType>,
            &MethodInfo,
        ) -> *mut SystemType;
        let make_method: MakeType = transmute((*make_generic_type).method_ptr);
        let object_list_type: *mut SystemType = make_method(
            list_type as *mut SystemType,
            type_array as _,
            &*make_generic_type,
        );

        struct SystemType {
            _obj: Il2CppObject,
            type_handle: *mut Il2CppType,
        }
        let object_list_type = (object_list_type as *mut SystemType).read().type_handle;
        let object_list_class = il2cpp_class_from_type(object_list_type);
        println!(
            "object_list_class: {}.{}",
            CStr::from_ptr(object_list_class.read().namespaze)
                .to_str()
                .unwrap(),
            CStr::from_ptr(object_list_class.read().name)
                .to_str()
                .unwrap(),
        );
        il2cpp_runtime_class_init(object_list_class);
        let object_list_ctor =
            il2cpp_class_get_method_from_name(object_list_class, ".ctor\0".as_ptr(), 0);
        let object_list = il2cpp_object_new(object_list_class);
        il2cpp_runtime_invoke(
            object_list_ctor,
            object_list as *mut c_void,
            null_mut(),
            &mut exc as _,
        );
        if exc != std::ptr::null_mut() {
            println!("Failed to invoke object_list ctor");
            print_exception(exc);
            return;
        }

        // let set_name = il2cpp_class_get_method_from_name(
        //     unityengine_object_class,
        //     "set_name\0".as_ptr(),
        //     1,
        // );

        let file_contents = std::fs::read_to_string("shit.txt").unwrap();
        let line = file_contents.lines().next().unwrap();
        println!("{}..{}", &line[..50], &line[line.len() - 50..]);
        // let line = "Lg==";
        let serialized_program_string = CString::new(line).unwrap();
        let string = il2cpp_string_new(serialized_program_string.as_ptr() as *const u8);

        struct SerializedUdonProgramAsset {
            _obj: Il2CppObject,
            _m_cached_ptr: *mut c_void,
            serialized_program_bytes_string: *const Il2CppString,
            program_unity_engine_objects: *const Il2CppObject,
            serialization_data_format: i32,
        }
        let serialized_udon_program_asset =
            serialized_udon_program_asset as *mut SerializedUdonProgramAsset;
        (*serialized_udon_program_asset).serialized_program_bytes_string = string;
        (*serialized_udon_program_asset).program_unity_engine_objects = object_list;
        (*serialized_udon_program_asset).serialization_data_format = 1;

        // let name = il2cpp_string_new("Fuck\0".as_ptr() as *const u8);
        // il2cpp_runtime_invoke(
        //     set_name,
        //     serialized_udon_program_asset as *mut c_void,
        //     [name as *mut _].as_ptr(),
        //     &mut exc as _,
        // );
        // if exc != null_mut() {
        //     println!("Failed to invoke set_name");
        //     print_exception(exc);
        //     return;
        // }

        let udon_program = il2cpp_runtime_invoke(
            retrieve_program,
            serialized_udon_program_asset as *mut c_void,
            null_mut(),
            &mut exc as _,
        );
        if exc != null_mut() {
            println!("Failed to invoke retrieve_program");
            print_exception(exc);
            return;
        }

        let udon_program = &*(udon_program as *const UdonProgram);
        println!("identifier: {}", *udon_program.instruction_set_identifier);
        println!("version: {}", udon_program.instruction_set_version);
        // let interpreter = load_program(array, heap);

        // il2cpp_shutdown();
    }
}

fn print_exception(exc: *const Il2CppException) {
    unsafe {
        let class = &*(*exc)._obj.klass;
        let namespace = CStr::from_ptr(class.namespaze).to_str().unwrap();
        let name = CStr::from_ptr(class.name).to_str().unwrap();
        println!("{}.{}: {}", namespace, name, *(*exc).message);

        if (*exc).inner_ex != null_mut() {
            println!("Inner exception:");
            print_exception((*exc).inner_ex);
        }

        if (*exc).trace_ips != null_mut() {
            for method in (*(*exc).trace_ips).as_slice() {
                let method = &**method;
                let class = &*method.class;
                let method_name = CStr::from_ptr(method.name).to_str().unwrap();
                let namespace = if class.namespaze != null_mut() {
                    CStr::from_ptr(class.namespaze).to_str().unwrap()
                } else {
                    "root"
                };
                let class_name = CStr::from_ptr(class.name).to_str().unwrap();
                println!("\tat {}.{}: {}", namespace, class_name, method_name);
            }
        }
    }
}
