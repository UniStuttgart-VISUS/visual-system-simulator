use std::{borrow::Borrow, default, mem::{self, size_of}, os::raw::{c_char, c_void}};
use vss::*;
use vss::gfx::Factory;

pub type OpenXRPtr = *mut c_void;


extern "C" {
    fn openxr_new(openxr: *mut OpenXRPtr) -> *const c_char;
    fn openxr_init(openxr: OpenXRPtr) -> *const c_char;
    fn openxr_create_session(openxr: OpenXRPtr) -> *const c_char;
    fn openxr_get_surfaces(
        openxr: OpenXRPtr,
        surfaces: *mut *mut u32,
        surfaces_size: *mut u32,
        surface_width: *mut u32,
        surface_height: *mut u32,
    ) -> *const c_char;
}


pub struct OpenXR {
    openxr: OpenXRPtr,
    render_targets_color: Vec<RenderTargetColor>,
}

impl OpenXR {
    pub fn new() -> Self {
        let mut openxr = std::ptr::null_mut();
        unsafe { openxr_new(&mut openxr as *mut *mut _)};
        OpenXR{
            openxr,
            render_targets_color: Vec::new()
        }
    }

    pub fn initialize(&self){
        unsafe {
            openxr_init(self.openxr);
        }
    }

    pub fn create_session(&self, surface: &Surface){

        //TODO do
        unsafe {
            openxr_create_session(self.openxr);            
        }

    }
    pub fn create_render_targets(&mut self, surface: &Surface) -> (u32, u32){

        let mut render_targets_size = 0u32;
        let mut surface_width = 0u32;
        let mut surface_height = 0u32;
        let mut render_targets = std::ptr::null_mut::<u32>();

        unsafe {
            openxr_get_surfaces(
                self.openxr,
                &mut render_targets as *mut *mut _,
                &mut render_targets_size as *mut _,
                &mut surface_width as *mut _,
                &mut surface_height as *mut _,
            );

        }

        let render_targets =
            unsafe { std::slice::from_raw_parts(render_targets, render_targets_size as usize) };

        let mut factory = window.factory().borrow_mut();
        for render_target in render_targets {
            let color_texture =texture_from_id_and_size::<ColorFormat>(
                *render_target,
                surface_width,
                surface_height,
            );
            self.render_targets_color.push(factory.view_texture_as_render_target(&color_texture, 0, None).unwrap());
        }
        (surface_width, surface_height)
    }
}

