use std::io::Cursor;
use std::path::Path;

/// Converts a struct to `&[u8]`.
///
/// # Safety
/// The function is marked unsafe because any padding bytes in the type may be uninitialized memory (giving undefined behavior).
/// It is safe (for sure) if the input argument is a struct with #[repr(packed)].
pub unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::std::slice::from_raw_parts((p as *const T) as *const u8, ::std::mem::size_of::<T>())
}

pub type LoadFn = Box<dyn Fn(&str) -> Cursor<Vec<u8>>>;
static mut LOAD_FN: Option<LoadFn> = None;

pub fn set_load(load_fn: LoadFn) {
    unsafe { LOAD_FN = Some(load_fn) };
}

pub fn load<P: AsRef<Path>>(path: P) -> Cursor<Vec<u8>> {
    if let Some(load_fn) = unsafe { &LOAD_FN } {
        load_fn(Path::new("").join(&path).to_str().unwrap())
    } else {
        panic!("load_fn not set");
    }
}
