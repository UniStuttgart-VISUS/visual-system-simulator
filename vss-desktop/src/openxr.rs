use std::os::raw::{c_char, c_void};

pub type OpenXRPtr = *mut c_void;


extern "C" {
    fn openxr_new(openxr: *mut OpenXRPtr) -> *const c_char;
    fn openxr_init(openxr: OpenXRPtr) -> *const c_char;
}


pub struct OpenXR {
    openxr: OpenXRPtr
}

impl OpenXR {
    pub fn new() -> Self {
        let mut openxr = std::ptr::null_mut();
        unsafe { openxr_new(&mut openxr as *mut *mut _)};
        OpenXR{
            openxr
        }
    }

    pub fn initialize(&self){
        unsafe {
            openxr_init(self.openxr);
        }
    }
}