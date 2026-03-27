//! Core types, errors, and traits for the recamera SDK.
//!
//! This crate provides the foundational building blocks shared by all other
//! crates in the `recamera-rs` workspace, including a unified [`Error`] type
//! and common data structures such as [`ImageFormat`], [`Resolution`], and
//! [`FrameData`].

pub mod error;
pub mod types;

pub use error::{Error, Result};
pub use types::{FrameData, ImageFormat, Resolution};
