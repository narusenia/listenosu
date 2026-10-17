fn main() {
    // Embed the icon so Explorer and the taskbar show it for listenosu.exe.
    if std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows") {
        winresource::WindowsResource::new()
            .set_icon("../../assets/icons/listenosu.ico")
            .compile()
            .expect("failed to embed the exe icon");
    }
}
