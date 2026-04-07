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
//! The vendor `.so` libraries are loaded at runtime via `dlopen` with
//! `RTLD_LAZY | RTLD_GLOBAL`. This is required on musl-based systems
//! (like reCamera-OS) where the vendor libraries lack proper `DT_NEEDED`
//! entries for their transitive dependencies. `RTLD_LAZY` defers symbol
//! resolution until first use, and `RTLD_GLOBAL` makes each library's
//! exports visible to subsequently loaded libraries.
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
