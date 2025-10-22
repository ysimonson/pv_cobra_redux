
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    if !Path::new("resources/cobra/.git").exists() {
        let _ = Command::new("git")
            .args(&["submodule", "update", "--init"])
            .status();
    }

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let (lib_dir, lib_ext) = match (target_os.as_str(), target_arch.as_str()) {
        ("linux", "x86_64") => ("linux/x86_64", "so"),
        ("linux", "aarch64") => ("raspberry-pi/cortex-a76-aarch64", "so"),
        ("macos", "x86_64") => ("mac/x86_64", "dylib"),
        ("macos", "aarch64") => ("mac/arm64", "dylib"),
        _ => panic!("Unsupported target: {target_os} {target_arch}"),
    };
    let lib_name = format!("libpv_cobra.{}", lib_ext);

    let src_lib_path = Path::new("resources/cobra/lib")
        .join(lib_dir)
        .join(&lib_name);
    if !src_lib_path.exists() {
        panic!("Expected library not found at {:?}", src_lib_path);
    }
    let dst_lib_path = out_dir.join(&lib_name);
    fs::copy(&src_lib_path, &dst_lib_path)
        .unwrap_or_else(|e| panic!("Failed to copy {:?} to {:?}: {}", src_lib_path, dst_lib_path, e));

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=dylib=pv_cobra");

    bindgen::Builder::default()
        .header("resources/cobra/include/picovoice.h")
        .header("resources/cobra/include/pv_cobra.h")
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings");

    println!("cargo:rerun-if-changed={}", src_lib_path.display());
    println!("cargo:rerun-if-changed=resources/cobra/include/picovoice.h");
    println!("cargo:rerun-if-changed=resources/cobra/include/pv_cobra.h");
}
