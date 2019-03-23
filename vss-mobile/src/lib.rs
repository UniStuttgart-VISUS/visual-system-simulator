extern crate vss;

#[cfg(target_os = "android")]
extern crate jni;
#[cfg(target_os = "android")]
extern crate android_glue;
#[cfg(target_os = "android")]
extern crate android_injected_glue;

#[cfg(target_os="android")]
mod camera_ffi;
 
//atm neededd to export function
#[cfg(target_os = "android")]
pub use android::Java_de_uni_1stuttgart_vis_fist_activities_CoreActivity_postData;
#[cfg(target_os = "android")]
pub use android::Java_de_uni_1stuttgart_vis_fist_activities_CoreActivity_postConfig;
 
/// Program entry point for Android.
#[cfg(target_os = "android")]
#[inline(never)]
#[no_mangle]
pub extern "C" fn android_main(app: *mut ()) {
    android_injected_glue::android_main2(app as *mut _, move |_, _| start_android());
}
 
/// (Inner) program entry point for Android.
#[cfg(target_os = "android")]
pub fn main() {
    let mut config = config::get_default_config();
    config.input_file = String::from("flowers.png");
    config.loop_provider = String::from("camera");
    config.special_loop_provider_set = true;
 
     
      let (mut device, mut pipeline) = config.build();

    let mut done = false;
    while !done {
        device.begin_frame();
        pipeline.render(&mut device);
        device.end_frame(&mut done);
    }
}
