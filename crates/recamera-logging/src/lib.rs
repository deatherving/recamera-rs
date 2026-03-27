//! Logging utilities for the recamera SDK.
//!
//! This crate provides a thin wrapper around [`tracing`] that makes it easy to
//! initialise structured logging for any application built on top of the
//! recamera SDK.
//!
//! # Quick start
//!
//! ```rust
//! use recamera_logging::{LogConfig, init};
//!
//! // Use defaults: INFO level, stdout only, no file output.
//! let config = LogConfig::default();
//! init(&config).expect("failed to initialise logging");
//! ```

use std::path::PathBuf;

use recamera_core::Result;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Log verbosity level.
///
/// Maps directly to the equivalent [`tracing::Level`] variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// The most verbose level — typically only useful during development.
    Trace,
    /// Verbose diagnostic information.
    Debug,
    /// General informational messages.
    Info,
    /// Potential issues that deserve attention.
    Warn,
    /// Hard failures that must be addressed.
    Error,
}

impl LogLevel {
    /// Returns the level as a static string suitable for [`EnvFilter`] directives.
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }
}

/// Configuration for the logging subsystem.
///
/// Use [`Default`] to get a sensible starting point and then customise
/// individual fields as needed.
///
/// # Examples
///
/// ```rust
/// use recamera_logging::{LogConfig, LogLevel};
///
/// let config = LogConfig {
///     level: LogLevel::Debug,
///     stdout: true,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// The minimum log level that will be emitted.
    pub level: LogLevel,
    /// Optional path to a directory where log files are written.
    ///
    /// When set, a rolling log file named `recamera.log` is created inside the
    /// given directory using [`tracing_appender`].
    pub output_path: Option<PathBuf>,
    /// Whether to emit log records to stdout.
    pub stdout: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            output_path: None,
            stdout: true,
        }
    }
}

/// Initialise the global tracing subscriber according to `config`.
///
/// This function should be called **once** early in the program's lifetime.
/// Calling it a second time will return an error because the global subscriber
/// has already been set.
///
/// # Errors
///
/// Returns [`recamera_core::Error::Config`] if the tracing subscriber cannot be
/// installed (e.g. it was already initialised).
pub fn init(config: &LogConfig) -> Result<()> {
    let env_filter = EnvFilter::new(config.level.as_str());

    let registry = tracing_subscriber::registry().with(env_filter);

    match (&config.output_path, config.stdout) {
        (Some(path), true) => {
            let file_appender =
                tracing_appender::rolling::daily(path, "recamera.log");
            let file_layer = fmt::layer().with_writer(file_appender).with_ansi(false);
            let stdout_layer = fmt::layer().with_writer(std::io::stdout);
            registry
                .with(file_layer)
                .with(stdout_layer)
                .try_init()
                .map_err(|e| recamera_core::Error::Config(e.to_string()))?;
        }
        (Some(path), false) => {
            let file_appender =
                tracing_appender::rolling::daily(path, "recamera.log");
            let file_layer = fmt::layer().with_writer(file_appender).with_ansi(false);
            registry
                .with(file_layer)
                .try_init()
                .map_err(|e| recamera_core::Error::Config(e.to_string()))?;
        }
        (None, true) => {
            let stdout_layer = fmt::layer().with_writer(std::io::stdout);
            registry
                .with(stdout_layer)
                .try_init()
                .map_err(|e| recamera_core::Error::Config(e.to_string()))?;
        }
        (None, false) => {
            // No output layers — install the registry alone so the subscriber
            // is set but nothing is emitted.
            registry
                .try_init()
                .map_err(|e| recamera_core::Error::Config(e.to_string()))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let config = LogConfig::default();
        assert_eq!(config.level, LogLevel::Info);
        assert!(config.output_path.is_none());
        assert!(config.stdout);
    }

    #[test]
    fn log_level_filter_strings() {
        assert_eq!(LogLevel::Trace.as_str(), "trace");
        assert_eq!(LogLevel::Debug.as_str(), "debug");
        assert_eq!(LogLevel::Info.as_str(), "info");
        assert_eq!(LogLevel::Warn.as_str(), "warn");
        assert_eq!(LogLevel::Error.as_str(), "error");
    }

    #[test]
    fn config_with_output_path() {
        let config = LogConfig {
            level: LogLevel::Debug,
            output_path: Some(PathBuf::from("/tmp/logs")),
            stdout: false,
        };
        assert_eq!(config.level, LogLevel::Debug);
        assert_eq!(config.output_path.as_deref(), Some(std::path::Path::new("/tmp/logs")));
        assert!(!config.stdout);
    }

    #[test]
    fn log_level_clone_and_copy() {
        let level = LogLevel::Warn;
        let cloned = level.clone();
        let copied = level;
        assert_eq!(level, cloned);
        assert_eq!(level, copied);
    }
}
