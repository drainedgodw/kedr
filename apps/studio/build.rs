#[cfg(windows)]
#[path = "src/icon_win.rs"]
mod icon_win;
#[cfg(windows)]
fn main() {
    let icon = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("codecedar.ico");
    std::fs::write(&icon, icon_win::ICO).expect("write CodeCedar icon");
    winres::WindowsResource::new()
        .set_icon(icon.to_str().unwrap())
        .set("ProductName", "CodeCedar")
        .set("FileDescription", "CodeCedar — Kedr preview")
        .compile()
        .expect("compile Windows icon resource");
    println!("cargo:rerun-if-changed=src/icon_win.rs");
}
#[cfg(not(windows))]
fn main() {}
