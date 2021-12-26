use libc::c_void;

/* Note: Blittable */
pub struct Span<T> {
    pub pinnable: *const c_void,
    pub byte_offset: *const T,
    pub length: i32,
}

impl<T> Span<T> {
    pub fn new(slice: &[T]) -> Span<T> {
        Span {
            pinnable: 0 as *const c_void,
            byte_offset: slice.as_ptr(),
            length: slice.len() as i32,
        }
    }
}
