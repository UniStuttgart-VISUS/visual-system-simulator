use std::os::raw::{c_char, c_void};
use vss::*;
use vss::gfx::Factory;
//use cgmath::{Matrix4, Vector3};

pub type OpenXRPtr = *mut c_void;


extern "C" {
    fn openxr_new(openxr: *mut OpenXRPtr) -> *const c_char;
    fn openxr_init(openxr: OpenXRPtr) -> *const c_char;
    fn openxr_create_session(openxr: OpenXRPtr) -> *const c_char;
    fn openxr_poll_events(openxr: OpenXRPtr, exit: *mut bool)-> *const c_char;
    fn openxr_get_surfaces(
        openxr: OpenXRPtr,
        surfaces: *mut *mut u32,
        surfaces_size: *mut u32,
        surface_width: *mut u32,
        surface_height: *mut u32,
    ) -> *const c_char;
    fn openxr_acquire_swapchain_images(openxr: OpenXRPtr, swapchain_index: *mut u32)-> *const c_char;
    fn openxr_begin_frame_sync(openxr: OpenXRPtr, is_available: *mut bool)-> *const c_char;
    fn openxr_end_frame(openxr: OpenXRPtr)-> *const c_char;    
}

pub struct OpenXR {
    openxr: OpenXRPtr,
    render_targets_color: Vec<RenderTargetColor>,
    render_targets_depth: Vec<RenderTargetDepth>
}

impl OpenXR {
    pub fn new() -> Self {
        let mut openxr = std::ptr::null_mut();
        unsafe { openxr_new(&mut openxr as *mut *mut _)};
        OpenXR{
            openxr,
            render_targets_color: Vec::new(),
            render_targets_depth: Vec::new()
        }
    }

    pub fn initialize(&self){
        unsafe {
            openxr_init(self.openxr);
        }
    }

    pub fn create_session(&self, window: &Window){
        unsafe {
            openxr_create_session(self.openxr);            
        }
    }

    pub fn poll_events(&self) -> bool{
        let mut exit = false;
        unsafe { openxr_poll_events(self.openxr, &mut exit as *mut _) };

        exit
    }

    pub fn create_render_targets(&mut self, window: &Window) -> (u32, u32){

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
            println!("color_texture {:?} from target {:?}", color_texture, render_target);
            self.render_targets_color.push(factory.view_texture_as_render_target(&color_texture, 0, None).unwrap());
            self.render_targets_depth.push(factory.create_depth_stencil(surface_width as u16, surface_height as u16).unwrap().2);
        }
        (surface_width, surface_height)
    }

    pub fn begin_frame_sync(&self) -> bool{
        let mut is_available = false;
        unsafe { openxr_begin_frame_sync(self.openxr, &mut is_available as *mut _) };

        is_available
    }

    pub fn get_current_render_target(&self) -> (RenderTargetColor, RenderTargetDepth){
        let mut current_swap_chain_index = 0u32;
        unsafe {
            openxr_acquire_swapchain_images(
                self.openxr,
                &mut current_swap_chain_index as *mut _
            )
        };

        return(self.render_targets_color[0].clone(), self.render_targets_depth[0].clone());
    }

    pub fn end_frame(&self) {
        unsafe { openxr_end_frame(self.openxr) };
    }
}

