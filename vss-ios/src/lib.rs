#[cfg(target_os = "ios")]
mod bridge;
#[cfg(target_os = "ios")]
mod frame;

// Keeping a host-compilable crate makes workspace tooling useful without an iOS SDK.
#[cfg(not(target_os = "ios"))]
pub fn ios_target_required() {}

