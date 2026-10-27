//! Embeds the Windows icon resource into `sdroxide.exe`.

fn main() {
    // The Windows installer's Start Menu and desktop shortcuts are advertised,
    // so Windows Installer draws their icon from the executable rather than
    // from a separate `.ico`. Embed the Brown icon into the exe so those
    // shortcuts show it. winres is a no-op on every non-Windows target.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let manifest = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        winres::WindowsResource::new()
            .set_icon(manifest.join("sdroxide-brown.ico").to_str().unwrap())
            .compile()
            .expect("embed the Windows icon resource");
    }
}
