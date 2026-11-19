// Embed the Windows app manifest with the MSVC linker (no extra crate). Without it Chromium
// can't see the real Windows version and the GPU process crashes (found in P0-02).
fn main() {
    println!("cargo:rerun-if-changed=app.manifest");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        let dir = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
        let path = std::path::Path::new(&dir).join("app.manifest");
        println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}",
            path.display()
        );
    }
}
