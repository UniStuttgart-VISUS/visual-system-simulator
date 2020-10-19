pub(crate) const COMMON_CODE: &'static str = include_str!("common.glsl");

//TODO: move to node/mod.rs
macro_rules! include_glsl {
    ($file: expr) => {{
        use glsl_include::Context;

        let version = if cfg!(target_os = "android") {
            "#version 300 es\nprecision mediump float;"
        } else {
            "#version 410"
        };
        let code = format!("{}\n{}", version, include_str!($file));

        Context::new()
            .include("common.glsl", &COMMON_CODE)
            .expand(code)
            .unwrap().as_bytes().to_vec()
    }};
}
