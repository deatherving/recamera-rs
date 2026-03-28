//! FFI bindings for the Sophgo SG2002 CVI vendor libraries.
//!
//! This crate provides low-level, `unsafe` FFI bindings to the CVI multimedia
//! pipeline libraries shipped in the reCamera-OS SDK. It covers:
//!
//! - **SYS** — system init, channel binding
//! - **VB** — video buffer pool management
//! - **VI** — video input (camera sensor)
//! - **VPSS** — video processing subsystem
//! - **VENC** — video encoding (H.264, H.265, JPEG)
//!
//! NPU inference bindings (`CVI_NN_*`) are not yet included because the
//! cviruntime headers are not part of the current reCamera-OS SDK release.
//!
//! # Regenerating bindings
//!
//! ```sh
//! SDK_PATH=./sdk/sg2002_recamera_emmc ./scripts/generate-bindings.sh
//! ```
//!
//! # Linking
//!
//! On `riscv64` targets the build script looks for `SG200X_SDK_PATH` and emits
//! the appropriate `cargo:rustc-link-lib` directives. On other targets linking
//! is skipped, allowing the crate to compile anywhere.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(improper_ctypes)]

mod bindings;
pub use bindings::*;
