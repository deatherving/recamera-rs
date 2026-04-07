use std::path::PathBuf;

fn main() {
    // Allow overriding the SDK library path via environment variable.
    // This is required when recamera-rs is used as a git dependency,
    // since the sdk/ directory is gitignored and not available in
    // Cargo's git checkout.
    //
    // Usage:  CVI_MPI_LIB_DIR=/path/to/cvi_mpi/lib cargo build
    let lib_dir = if let Ok(dir) = std::env::var("CVI_MPI_LIB_DIR") {
        PathBuf::from(dir)
    } else {
        [
            env!("CARGO_MANIFEST_DIR"),
            "..",
            "..",
            "sdk",
            "sg2002_recamera_emmc",
            "cvi_mpi",
            "lib",
        ]
        .iter()
        .collect()
    };

    println!("cargo:rerun-if-env-changed=CVI_MPI_LIB_DIR");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    for lib in ["sys", "vi", "vpss", "venc", "cviruntime"] {
        println!("cargo:rustc-link-lib=dylib={lib}");
    }

    // The vendor .so files have transitive dependencies on each other and
    // on system libraries (libisp, libvo, libatomic, etc.) that are only
    // present on the device. Tell the linker not to error on unresolved
    // symbols inside shared libraries — they will be resolved at runtime
    // by the device's dynamic linker.
    println!("cargo:rustc-link-arg=-Wl,--allow-shlib-undefined");
}
