//! Build script to copy the compiled dylib to the location where dylint expects it
//!
//! Dylint expects libraries at:
//! `target/dylint/libraries/<toolchain>/release/lib<name>@<toolchain><dll_suffix>`
//!
//! This script automates the copy after compilation.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Get the output directory (e.g., lints/target/release/)
    let out_dir = env::var("OUT_DIR").unwrap();
    let profile = if out_dir.contains("release") {
        "release"
    } else {
        "debug"
    };

    // Determine the toolchain (read from rust-toolchain.toml or RUSTUP_TOOLCHAIN)
    let toolchain = env::var("RUSTUP_TOOLCHAIN")
        .or_else(|_| env::var("RUSTC_VERSION"))
        .unwrap_or_else(|_| "nightly-2025-10-16".to_string());

    // Get host triple
    let host = env::var("HOST").unwrap();

    // DLL suffix
    let dll_suffix = if cfg!(target_os = "macos") {
        ".dylib"
    } else if cfg!(target_os = "windows") {
        ".dll"
    } else {
        ".so"
    };

    // Source: lints/target/<profile>/liboxidize_pdf_lints@<toolchain>-<host>.dylib
    let lib_name = format!("liboxidize_pdf_lints@{}-{}{}", toolchain, host, dll_suffix);
    let source = PathBuf::from("target").join(profile).join(&lib_name);

    // Destination: ../target/dylint/libraries/<toolchain>-<host>/<profile>/
    let dest_dir = PathBuf::from("..")
        .join("target")
        .join("dylint")
        .join("libraries")
        .join(format!("{}-{}", toolchain, host))
        .join(profile);

    let dest = dest_dir.join(&lib_name);

    // Create destination directory if it doesn't exist
    if let Err(e) = fs::create_dir_all(&dest_dir) {
        eprintln!("Warning: Failed to create dylint directory: {}", e);
        return;
    }

    // Copy the dylib (ignore errors if source doesn't exist yet)
    if source.exists() {
        if let Err(e) = fs::copy(&source, &dest) {
            eprintln!("Warning: Failed to copy dylib to dylint location: {}", e);
        } else {
            println!("cargo:warning=Copied {} to {}", source.display(), dest.display());
        }
    }

    // Tell Cargo to rerun this script if the dylib changes
    println!("cargo:rerun-if-changed=target/{}/{}", profile, lib_name);
}
