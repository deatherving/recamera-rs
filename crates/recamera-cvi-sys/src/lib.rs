//! FFI bindings for the Sophgo SG2002 CVI vendor libraries.
//!
//! This crate provides low-level, `unsafe` FFI bindings to the CVI multimedia
//! pipeline and neural-network runtime libraries shipped in the SG200X SDK.
//!
//! # Current status
//!
//! The bindings module is currently empty. To generate the full FFI bindings
//! from the vendor SDK headers, run:
//!
//! ```sh
//! ./scripts/generate-bindings.sh
//! ```
//!
//! # Linking
//!
//! On `riscv64gc-unknown-linux-gnu` targets the build script will look for the
//! `SG200X_SDK_PATH` environment variable and emit the appropriate
//! `cargo:rustc-link-lib` directives. On all other targets linking is skipped,
//! allowing the crate to compile (but not run) anywhere.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

mod bindings;
