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
    link_clang_runtime_on_macos();
    tauri_build::try_build(attributes).expect("failed to run tauri-build");
}

/// whisper.cpp's Metal code checks the macOS version at run time (`@available`),
/// which calls `__isPlatformVersionAtLeast` from clang's runtime library. Rust
/// links without that library, so the release build (macOS 10.15 and later,
/// where those checks are not compiled away) fails with an undefined symbol.
/// Link the copy that ships with the Xcode clang that builds whisper.cpp.
fn link_clang_runtime_on_macos() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let resource_dir = std::process::Command::new("xcrun")
        .args(["clang", "--print-resource-dir"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string());
    let Some(resource_dir) = resource_dir else {
        println!("cargo:warning=could not find clang's resource folder; whisper.cpp may not link");
        return;
    };
    let runtime_dir = std::path::Path::new(&resource_dir)
        .join("lib")
        .join("darwin");
    if !runtime_dir.join("libclang_rt.osx.a").exists() {
        println!(
            "cargo:warning=libclang_rt.osx.a is not in {}; whisper.cpp may not link",
            runtime_dir.display()
        );
        return;
    }
    println!("cargo:rustc-link-search=native={}", runtime_dir.display());
    println!("cargo:rustc-link-lib=static=clang_rt.osx");
}
