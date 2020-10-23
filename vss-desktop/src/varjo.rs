use std::os::raw::{c_char, c_void};

use vss::*;

type VarjoPtr = *mut c_void;

#[repr(C)]
#[derive(Clone)]
struct VarjoRenderTarget {
    pub texture_id: u32,
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

    pub fn render_targets(&self) -> Vec<u32> {
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
        let render_targets =
            unsafe { std::slice::from_raw_parts(render_targets, render_targets_size as usize) };

        let mut textures = Vec::new();
        for render_target in render_targets {
            textures.push(texture_from_id_and_size::<ColorFormat>(
                render_target.texture_id,
                render_target.width,
                render_target.height,
            ));
            //XXX: Create RenderTarget and ShaderResourceView
        }

        vec![0u32] //XXX: convert to gfx
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
