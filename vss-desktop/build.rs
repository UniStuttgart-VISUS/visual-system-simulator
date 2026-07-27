fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        winresource::WindowsResource::new()
            .set_icon("packaging/icons/windows/app.ico")
            .compile()
            .expect("failed to embed the Windows application icon");
    }
}
