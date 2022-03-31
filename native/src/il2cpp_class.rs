use std::os::raw::c_char;

use libc::c_void;

pub struct Il2CppImage;

pub enum Il2CppTypeEnum {
    END        = 0x00,       /* End of List */
    VOID       = 0x01,
    BOOLEAN    = 0x02,
    CHAR       = 0x03,
    I1         = 0x04,
    U1         = 0x05,
    I2         = 0x06,
    U2         = 0x07,
    I4         = 0x08,
    U4         = 0x09,
    I8         = 0x0a,
    U8         = 0x0b,
    R4         = 0x0c,
    R8         = 0x0d,
    STRING     = 0x0e,
    PTR        = 0x0f,       /* arg: <type> token */
    BYREF      = 0x10,       /* arg: <type> token */
    VALUETYPE  = 0x11,       /* arg: <type> token */
    CLASS      = 0x12,       /* arg: <type> token */
    VAR        = 0x13,       /* Generic parameter in a generic type definition, represented as number (compressed unsigned integer) number */
    ARRAY      = 0x14,       /* type, rank, boundsCount, bound1, loCount, lo1 */
    GENERICINST = 0x15,     /* <type> <type-arg-count> <type-1> \x{2026} <type-n> */
    TYPEDBYREF = 0x16,
    I          = 0x18,
    U          = 0x19,
    FNPTR      = 0x1b,        /* arg: full method signature */
    OBJECT     = 0x1c,
    SZARRAY    = 0x1d,       /* 0-based one-dim-array */
    MVAR       = 0x1e,       /* Generic parameter in a generic method definition, represented as number (compressed unsigned integer)  */
    CMOD_REQD  = 0x1f,       /* arg: typedef or typeref token */
    CMOD_OPT   = 0x20,       /* optional arg: typedef or typref token */
    INTERNAL   = 0x21,       /* CLR internal type */

    MODIFIER   = 0x40,       /* Or with the following types */
    SENTINEL   = 0x41,       /* Sentinel for varargs method signature */
    PINNED     = 0x45,       /* Local var that points to pinned object */

    ENUM       = 0x55        /* an enumeration */
}

pub struct Il2CppType {
    data: *const c_void,
    bitfield: u32,
}

impl Il2CppType {
    fn get_attrs(&self) -> u32 {
        (self.bitfield >> 0x00) & 0xffff
    }
    fn get_type(&self) -> u32 {
        (self.bitfield >> 0x10) & 0xff
    }
    fn get_num_mods(&self) -> u32 {
        (self.bitfield >> 0x18) & 0x3f
    }
    fn get_byref(&self) -> bool {
        (self.bitfield >> 0x1e) & 0x01 != 0
    }
    fn get_pinned(&self) -> bool {
        (self.bitfield >> 0x1f) & 0x01 != 0
    }
}

#[test]
fn assert_sizes() {
    assert_eq!(std::mem::size_of::<Il2CppType>(), 0x10);
}

pub struct Il2CppClass {
    image: *const Il2CppImage,
    gc_desc: *const c_void,
    pub name: *const c_char,
    pub namespaze: *const c_char,
    pub byval_arg: Il2CppType,
    pub this_arg: Il2CppType,
}
