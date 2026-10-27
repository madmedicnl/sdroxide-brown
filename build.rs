//! Embeds the Windows icon resource into `sdroxide.exe`.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // The installer's Start Menu and desktop shortcuts are advertised, so
        // Windows Installer draws their icon from the executable rather than a
        // .ico. Embed the Brown icon with `windres` (the MinGW resource
        // compiler) into a COFF object and hand that to the linker.
        //
        // MSYS2's `windres` passes backslash paths to the GCC preprocessor,
        // which reads them as escape sequences and fails with
        // "No such file or directory" (winres 0.1 died on exactly this). Every
        // path here is therefore spelled with forward slashes.
        let out_dir = std::env::var("OUT_DIR").unwrap().replace('\\', "/");
        let status = std::process::Command::new("windres")
            .arg("resource.rc")
            .arg("-O").arg("coff")
            .arg("-o").arg(format!("{}/resource.o", out_dir))
            .status()
            .expect("run windres");
        assert!(status.success(), "windres failed to compile resource.rc");
        println!("cargo:rustc-link-arg-bins={}/resource.o", out_dir);
    }
}
