use std::path::PathBuf;

fn main() {
    // Path to the CVI MPI vendor libraries shipped in the SDK.
    let sdk_lib_dir: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "..",
        "..",
        "sdk",
        "sg2002_recamera_emmc",
        "cvi_mpi",
        "lib",
    ]
    .iter()
    .collect();

    println!(
        "cargo:rustc-link-search=native={}",
        sdk_lib_dir.display()
    );

    // Vendor shared libraries required by the CVI multimedia pipeline.
    for lib in ["sys", "vi", "vpss", "venc", "cviruntime"] {
        println!("cargo:rustc-link-lib=dylib={lib}");
    }
}
