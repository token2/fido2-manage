fn main() {
    // Where libt2piv (the PIV library from token2-piv-tool) lives. Point T2PIV_LIB_DIR at the token2-piv-tool build
    // output (…/build/lib on Linux/macOS, …/build/lib/Release on Windows).
    if let Ok(dir) = std::env::var("T2PIV_LIB_DIR") {
        println!("cargo:rustc-link-search=native={dir}");
    }
    println!("cargo:rerun-if-env-changed=T2PIV_LIB_DIR");
    let name = std::env::var("T2PIV_LIB_NAME").unwrap_or_else(|_| {
        if cfg!(target_os = "windows") { "libt2piv".into() } else { "t2piv".into() }
    });
    println!("cargo:rustc-link-lib=dylib={name}");
    // libfido2 (fido2-manage fork, PC/SC enabled)
    if let Ok(dir) = std::env::var("FIDO2_LIB_DIR") {
        println!("cargo:rustc-link-search=native={dir}");
    }
    println!("cargo:rerun-if-env-changed=FIDO2_LIB_DIR");
    println!("cargo:rerun-if-env-changed=FIDO2_LINK_STATIC");
    let fido = std::env::var("FIDO2_LIB_NAME").unwrap_or_else(|_| "fido2".into());
    if std::env::var("FIDO2_LINK_STATIC").is_ok() {
        // Static-link libfido2 so there is no runtime .so resolution (avoids a
        // system libfido2 of a different version shadowing the bundled one).
        // A static archive does not pull its own dependencies, so link them here.
        println!("cargo:rustc-link-lib=static={fido}");
        for dep in ["cbor", "crypto", "z", "udev"] {
            println!("cargo:rustc-link-lib=dylib={dep}");
        }
        // PC/SC: pcsclite on Linux.
        println!("cargo:rustc-link-lib=dylib=pcsclite");
    } else {
        println!("cargo:rustc-link-lib=dylib={fido}");
    }
    tauri_build::build()
}
