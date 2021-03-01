use std::{borrow::Borrow, os::raw::{c_char, c_void}};
use vss::*;


pub type OpenXRPtr = *mut c_void;


extern "C" {
    fn openxr_new(openxr: *mut OpenXRPtr) -> *const c_char;
    fn openxr_init(openxr: OpenXRPtr) -> *const c_char;
    fn openxr_create_session(openxr: OpenXRPtr) -> *const c_char;

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

    pub fn create_session(&mut self, window: &Window){

        //TODO do

        let device = window.device().as_ptr();
        print!("Device ptr: {:p}",device);

        unsafe {
            openxr_create_session(self.openxr);
        }
    }
}