//! FFI bindings for the Sophgo SG2002 CVI vendor libraries.
//!
//! This crate provides low-level types, constants, and a runtime dynamic
//! loader for the CVI multimedia pipeline libraries shipped in the
//! reCamera-OS SDK. It covers:
//!
//! - **SYS** -- system init, channel binding
//! - **VB** -- video buffer pool management
//! - **VI** -- video input (camera sensor)
//! - **VPSS** -- video processing subsystem
//! - **VENC** -- video encoding (H.264, H.265, JPEG)
//! - **NN** -- NPU inference runtime (model loading, tensor I/O, forward pass)
//!
//! # Runtime loading
//!
//! The vendor `.so` libraries are **not** linked at compile time. Instead,
//! [`CviLibs::load`] opens them at runtime via `dlopen`, so `cargo build`
//! works on any host without the SDK installed. The actual shared objects
//! are only required on the reCamera device at runtime.
//!
//! # Regenerating bindings
//!
//! ```sh
//! SDK_PATH=./sdk/sg2002_recamera_emmc ./scripts/generate-bindings.sh
//! ```

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(improper_ctypes)]

mod bindings;
pub use bindings::*;

mod loader;
pub use loader::CviLibs;
