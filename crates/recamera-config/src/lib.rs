//! Configuration loading and validation for the recamera SDK.
//!
//! This crate provides helpers for loading TOML configuration files and
//! deserializing them into strongly-typed Rust structs via [`serde`].
//!
//! # Examples
//!
//! ```
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct AppConfig {
//!     name: String,
//!     port: u16,
//! }
//!
//! let config: AppConfig = recamera_config::from_str(r#"
//! name = "my-app"
//! port = 8080
//! "#).unwrap();
//!
//! assert_eq!(config.name, "my-app");
//! assert_eq!(config.port, 8080);
//! ```

use std::path::Path;

use recamera_core::{Error, Result};

/// Load a TOML configuration file from disk and deserialize it into `T`.
///
/// The file at `path` is read in its entirety and then parsed as TOML.
/// Any I/O or deserialization error is mapped to a [`recamera_core::Error`].
///
/// # Errors
///
/// Returns [`Error::Io`] if the file cannot be read, or [`Error::Config`] if
/// the contents are not valid TOML or do not match the target type `T`.
pub fn load<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let contents = std::fs::read_to_string(path)?;
    from_str(&contents)
}

/// Parse a TOML string and deserialize it into `T`.
///
/// This is useful when configuration text is already available in memory
/// (e.g. embedded in tests or fetched from a remote source).
///
/// # Errors
///
/// Returns [`Error::Config`] if the string is not valid TOML or does not
/// match the expected structure of `T`.
pub fn from_str<T: serde::de::DeserializeOwned>(s: &str) -> Result<T> {
    toml::from_str(s).map_err(|e| Error::Config(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::io::Write;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestConfig {
        name: String,
        value: i32,
    }

    #[test]
    fn parse_valid_toml() {
        let input = r#"
            name = "hello"
            value = 42
        "#;
        let config: TestConfig = from_str(input).unwrap();
        assert_eq!(
            config,
            TestConfig {
                name: "hello".into(),
                value: 42,
            }
        );
    }

    #[test]
    fn parse_invalid_toml_returns_error() {
        let input = "not valid [[[ toml !!!";
        let result: Result<TestConfig> = from_str(input);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Config(_)));
    }

    #[test]
    fn load_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        {
            let mut f = std::fs::File::create(&path).unwrap();
            writeln!(f, "name = \"from-file\"").unwrap();
            writeln!(f, "value = 99").unwrap();
        }
        let config: TestConfig = load(&path).unwrap();
        assert_eq!(config.name, "from-file");
        assert_eq!(config.value, 99);
    }

    #[test]
    fn load_missing_file_returns_error() {
        let result: Result<TestConfig> = load(Path::new("/nonexistent/path/config.toml"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, Error::Io(_)));
    }
}
