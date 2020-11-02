use std::os::raw::{c_char, c_void};

use vss::*;
use vss::gfx::Factory;

type VarjoPtr = *mut c_void;

#[repr(C)]
#[derive(Clone)]
struct VarjoRenderTarget {
    pub color_texture_id: u32,
    pub depth_texture_id: u32,
    pub width: u32,
    pub height: u32,
}

extern "C" {
    fn varjo_new(varjo: *mut VarjoPtr) -> *const c_char;
    fn varjo_render_targets(
        varjo: VarjoPtr,
        render_targets: *mut *mut VarjoRenderTarget,
        render_targets_size: *mut u32,
    ) -> *const c_char;
    fn varjo_begin_frame_sync(varjo: VarjoPtr) -> *const c_char;
    fn varjo_current_swap_chain_index(
        varjo: VarjoPtr,
        current_swap_chain_index: *mut u32,
    ) -> *const c_char;
    fn varjo_end_frame(varjo: VarjoPtr) -> *const c_char;
    fn varjo_drop(varjo: *mut VarjoPtr);
}

#[derive(Debug)]
pub struct VarjoErr(String);

fn try_fail(error: *const c_char) -> Result<(), VarjoErr> {
    if error == std::ptr::null_mut() {
        Ok(())
    } else {
        use std::ffi::CStr;
        let c_str: &CStr = unsafe { CStr::from_ptr(error) };
        let str_slice: &str = c_str.to_str().unwrap();

        Err(VarjoErr(str_slice.to_owned()))
    }
}

pub struct Varjo {
    varjo: VarjoPtr,
}

impl Varjo {
    pub fn new() -> Self {
        let mut varjo = std::ptr::null_mut();
        try_fail(unsafe { varjo_new(&mut varjo as *mut *mut _) }).unwrap();
        Self { varjo }
    }

    pub fn render_targets(&self, window: &Window) -> (RenderTargetColor, RenderTargetDepth) {
        let mut render_targets = std::ptr::null_mut::<VarjoRenderTarget>();
        let mut render_targets_size = 0u32;
        try_fail(unsafe {
            varjo_render_targets(
                self.varjo,
                &mut render_targets as *mut *mut _,
                &mut render_targets_size as *mut _,
            )
        })
        .unwrap();
        let mut current_swap_chain_index = 0u32;
        try_fail(unsafe {
            varjo_current_swap_chain_index(
                self.varjo,
                &mut current_swap_chain_index as *mut _,
            )
        })
        .unwrap();
        let render_targets =
            unsafe { std::slice::from_raw_parts(render_targets, render_targets_size as usize) };

        let mut textures = Vec::new();
        let mut depth_textures = Vec::new();
        for render_target in render_targets {
            textures.push(texture_from_id_and_size::<ColorFormat>(
                render_target.color_texture_id,
                render_target.width,
                render_target.height,
            ));
            depth_textures.push(depth_texture_from_id_and_size::<RenderTargetDepthFormat>(
                render_target.depth_texture_id,
                render_target.width,
                render_target.height,
            ));
        }
        let mut factory = window.factory().borrow_mut();
        //XXX: return all render targets and depth stencils at once
        return (factory.view_texture_as_render_target(&textures[current_swap_chain_index as usize], 0, None).unwrap(),
                factory.view_texture_as_depth_stencil(&depth_textures[current_swap_chain_index as usize], 0, None, gfx::texture::DepthStencilFlags::empty()).unwrap());
    }

    pub fn begin_frame_sync(&self) {
        try_fail(unsafe { varjo_begin_frame_sync(self.varjo) }).unwrap();
    }

    pub fn end_frame(&self) {
        try_fail(unsafe { varjo_end_frame(self.varjo) }).unwrap();
    }
}

impl Drop for Varjo {
    fn drop(&mut self) {
        unsafe {
            varjo_drop(&mut self.varjo as *mut *mut _);
        }
    }
}
