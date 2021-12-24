use libc::c_void;

/* Note: Blittable */
pub struct Span<'a, T> {
    pub pinnable: *const c_void,
    pub byte_offset: &'a T,
    pub length: i32,
}

impl<'a, T> Span<'a, T> {
    pub fn new(slice: &'a [T]) -> Span<'a, T> {
        Span {
            pinnable: 0 as *const c_void,
            byte_offset: &slice[0],
            length: slice.len() as i32,
        }
    }
}
