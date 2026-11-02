fn main() {
    // On Windows the app needs Common Controls v6 (dialogs use TaskDialogIndirect).
    // tauri-build embeds that manifest only into the app binary, so test binaries
    // die at start with STATUS_ENTRYPOINT_NOT_FOUND. Embed it through the linker
    // instead, which reaches every binary, and turn tauri-build's copy off so the
    // app does not get two. See tauri-apps/tauri#13419.
    let windows_msvc = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");

    let mut attributes = tauri_build::Attributes::new();
    if windows_msvc {
        attributes = attributes
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
        let manifest =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    }
    tauri_build::try_build(attributes).expect("failed to run tauri-build");
}
