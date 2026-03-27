//! Error types for the recamera SDK.

/// The primary error type for the recamera SDK.
///
/// All fallible operations across the SDK return this error type (or a variant
/// wrapped in [`Result`]). Each variant corresponds to a distinct subsystem so
/// callers can match on the kind of failure and handle it appropriately.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An I/O error originating from the standard library.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A configuration-related error (e.g. invalid or missing settings).
    #[error("config error: {0}")]
    Config(String),

    /// A camera subsystem error.
    #[error("camera error: {0}")]
    Camera(String),

    /// An inference engine error.
    #[error("inference error: {0}")]
    Inference(String),

    /// A serial-port communication error.
    #[error("serial error: {0}")]
    Serial(String),

    /// A storage subsystem error.
    #[error("storage error: {0}")]
    Storage(String),

    /// A general system-level error.
    #[error("system error: {0}")]
    System(String),
}

/// A convenience type alias for `std::result::Result<T, Error>`.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
        assert!(err.to_string().contains("file missing"));
    }

    #[test]
    fn display_variants() {
        let cases: Vec<(Error, &str)> = vec![
            (Error::Config("bad key".into()), "config error: bad key"),
            (Error::Camera("no device".into()), "camera error: no device"),
            (
                Error::Inference("model load".into()),
                "inference error: model load",
            ),
            (Error::Serial("timeout".into()), "serial error: timeout"),
            (Error::Storage("full".into()), "storage error: full"),
            (Error::System("oom".into()), "system error: oom"),
        ];
        for (err, expected) in cases {
            assert_eq!(err.to_string(), expected);
        }
    }

    #[test]
    fn result_alias_works() {
        fn ok_result() -> Result<u32> {
            Ok(42)
        }
        fn err_result() -> Result<u32> {
            Err(Error::System("fail".into()))
        }
        assert_eq!(ok_result().unwrap(), 42);
        assert!(err_result().is_err());
    }
}
