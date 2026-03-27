# recamera-rs SDK Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a modular Rust SDK for Seeed reCamera (SG2002) with camera capture, inference, UART, RS-485, storage, logging, config, and system utilities.

**Architecture:** Cargo workspace with individual crates + a facade `recamera` crate that re-exports via feature flags. FFI crates wrap vendor C libraries (cvi_mpi, cviruntime) using pre-generated bindings. Pure-Rust crates handle serial, storage, logging, config, and system.

**Tech Stack:** Rust (2021 edition), `thiserror`, `serde`/`toml`, `tracing`/`tracing-subscriber`/`tracing-appender`, `serialport`, `image`, `nix`

---

## File Map

```
recamera-rs/
  Cargo.toml                              # Workspace root
  .cargo/config.toml                      # Cross-compilation config
  crates/
    recamera-core/
      Cargo.toml
      src/lib.rs                          # Re-exports modules
      src/error.rs                        # Error enum + Result alias
      src/types.rs                        # ImageFormat, Resolution, FrameData
    recamera-logging/
      Cargo.toml
      src/lib.rs                          # LogConfig, init()
    recamera-config/
      Cargo.toml
      src/lib.rs                          # Config::load, Config::from_str, validate
    recamera-storage/
      Cargo.toml
      src/lib.rs                          # save_file, save_image, list_files, StorageInfo
    recamera-uart/
      Cargo.toml
      src/lib.rs                          # Uart struct, UartConfig, open/read/write
    recamera-rs485/
      Cargo.toml
      src/lib.rs                          # Rs485 struct, wraps Uart + GPIO direction
    recamera-system/
      Cargo.toml
      src/lib.rs                          # DeviceInfo, Led, reboot, uptime
    recamera-cvi-sys/
      Cargo.toml
      src/lib.rs                          # Re-export generated bindings
      src/bindings.rs                     # Pre-generated FFI bindings (placeholder)
      build.rs                            # Link-time SDK discovery
    recamera-camera/
      Cargo.toml
      src/lib.rs                          # Camera, Frame, FrameStream, CameraConfig
    recamera-infer/
      Cargo.toml
      src/lib.rs                          # Engine, Model, Output, ModelInfo
    recamera/
      Cargo.toml                          # Facade with feature flags
      src/lib.rs                          # Re-exports
  examples/
    capture_frame.rs                      # Camera capture example
    uart_echo.rs                          # UART echo test
    detect.rs                             # Camera -> inference pipeline
```

---

## Task 1: Workspace Root + recamera-core

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/recamera-core/Cargo.toml`
- Create: `crates/recamera-core/src/lib.rs`
- Create: `crates/recamera-core/src/error.rs`
- Create: `crates/recamera-core/src/types.rs`

- [ ] **Step 1: Create workspace root Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
    "crates/recamera-core",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
repository = "https://github.com/anthropics/recamera-rs"
rust-version = "1.75"
```

- [ ] **Step 2: Create recamera-core Cargo.toml**

Create `crates/recamera-core/Cargo.toml`:

```toml
[package]
name = "recamera-core"
description = "Core types, errors, and traits for the recamera SDK"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]
thiserror = "2"
```

- [ ] **Step 3: Write failing test for Error type**

Create `crates/recamera-core/src/error.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("camera error: {0}")]
    Camera(String),

    #[error("inference error: {0}")]
    Inference(String),

    #[error("serial error: {0}")]
    Serial(String),

    #[error("storage error: {0}")]
    Storage(String),

    #[error("system error: {0}")]
    System(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_converts() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
        assert!(err.to_string().contains("gone"));
    }

    #[test]
    fn camera_error_displays() {
        let err = Error::Camera("sensor not found".into());
        assert_eq!(err.to_string(), "camera error: sensor not found");
    }

    #[test]
    fn result_alias_works() {
        let ok: Result<u32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: Result<u32> = Err(Error::Config("bad".into()));
        assert!(err.is_err());
    }
}
```

- [ ] **Step 4: Write types module with tests**

Create `crates/recamera-core/src/types.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Rgb888,
    Jpeg,
    H264,
    Nv21,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Resolution {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

pub struct FrameData {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: ImageFormat,
    pub timestamp_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolution_new() {
        let r = Resolution::new(1920, 1080);
        assert_eq!(r.width, 1920);
        assert_eq!(r.height, 1080);
    }

    #[test]
    fn resolution_equality() {
        let a = Resolution::new(1280, 720);
        let b = Resolution::new(1280, 720);
        assert_eq!(a, b);
    }

    #[test]
    fn image_format_debug() {
        assert_eq!(format!("{:?}", ImageFormat::Jpeg), "Jpeg");
    }

    #[test]
    fn frame_data_construction() {
        let frame = FrameData {
            data: vec![0u8; 100],
            width: 640,
            height: 480,
            format: ImageFormat::Rgb888,
            timestamp_ms: 12345,
        };
        assert_eq!(frame.data.len(), 100);
        assert_eq!(frame.width, 640);
    }
}
```

- [ ] **Step 5: Create lib.rs to wire modules**

Create `crates/recamera-core/src/lib.rs`:

```rust
mod error;
mod types;

pub use error::{Error, Result};
pub use types::{FrameData, ImageFormat, Resolution};
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p recamera-core`
Expected: All 7 tests pass.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/recamera-core/
git commit -m "feat: add workspace root and recamera-core crate with error types and common types"
```

---

## Task 2: recamera-logging

**Files:**
- Create: `crates/recamera-logging/Cargo.toml`
- Create: `crates/recamera-logging/src/lib.rs`
- Modify: `Cargo.toml` (add to workspace members)

- [ ] **Step 1: Add to workspace members**

In root `Cargo.toml`, add `"crates/recamera-logging"` to `members`.

- [ ] **Step 2: Create recamera-logging Cargo.toml**

Create `crates/recamera-logging/Cargo.toml`:

```toml
[package]
name = "recamera-logging"
description = "Logging utilities for the recamera SDK"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]
recamera-core = { path = "../recamera-core" }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
tracing-appender = "0.2"
```

- [ ] **Step 3: Write logging module with tests**

Create `crates/recamera-logging/src/lib.rs`:

```rust
use recamera_core::{Error, Result};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone)]
pub struct LogConfig {
    pub level: LogLevel,
    pub output_path: Option<PathBuf>,
    pub stdout: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
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

impl LogLevel {
    fn as_filter_str(self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }
}

pub fn init(config: &LogConfig) -> Result<()> {
    let filter = EnvFilter::try_new(config.level.as_filter_str())
        .map_err(|e| Error::Config(format!("invalid log filter: {e}")))?;

    let subscriber = tracing_subscriber::fmt().with_env_filter(filter);

    if let Some(ref path) = config.output_path {
        let dir = path.parent().unwrap_or(path);
        let filename = path
            .file_name()
            .ok_or_else(|| Error::Config("log output_path has no filename".into()))?;

        let file_appender = tracing_appender::rolling::never(dir, filename);

        if config.stdout {
            let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
            // Note: _guard is leaked intentionally to keep the writer alive for the process lifetime.
            // In a real application, the guard should be held in main().
            std::mem::forget(_guard);
            subscriber
                .with_writer(non_blocking)
                .init();
        } else {
            let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
            std::mem::forget(_guard);
            subscriber
                .with_writer(non_blocking)
                .init();
        }
    } else {
        subscriber.init();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = LogConfig::default();
        assert_eq!(config.level, LogLevel::Info);
        assert!(config.output_path.is_none());
        assert!(config.stdout);
    }

    #[test]
    fn log_level_filter_str() {
        assert_eq!(LogLevel::Trace.as_filter_str(), "trace");
        assert_eq!(LogLevel::Error.as_filter_str(), "error");
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p recamera-logging`
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/recamera-logging/
git commit -m "feat: add recamera-logging crate with tracing-based log setup"
```

---

## Task 3: recamera-config

**Files:**
- Create: `crates/recamera-config/Cargo.toml`
- Create: `crates/recamera-config/src/lib.rs`
- Modify: `Cargo.toml` (add to workspace members)

- [ ] **Step 1: Add to workspace members**

In root `Cargo.toml`, add `"crates/recamera-config"` to `members`.

- [ ] **Step 2: Create recamera-config Cargo.toml**

Create `crates/recamera-config/Cargo.toml`:

```toml
[package]
name = "recamera-config"
description = "Configuration loading and validation for the recamera SDK"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]
recamera-core = { path = "../recamera-core" }
serde = { version = "1", features = ["derive"] }
toml = "0.8"
```

- [ ] **Step 3: Write config module with tests**

Create `crates/recamera-config/src/lib.rs`:

```rust
use recamera_core::{Error, Result};
use std::path::Path;

/// Load a TOML config file and deserialize into the given type.
///
/// The type `T` must implement `serde::de::DeserializeOwned`.
pub fn load<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let contents = std::fs::read_to_string(path).map_err(|e| {
        Error::Config(format!("failed to read config file {}: {e}", path.display()))
    })?;
    from_str(&contents)
}

/// Parse a TOML string into the given type.
pub fn from_str<T: serde::de::DeserializeOwned>(s: &str) -> Result<T> {
    toml::from_str(s).map_err(|e| Error::Config(format!("failed to parse config: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::io::Write;

    #[derive(Debug, Deserialize, PartialEq)]
    struct AppConfig {
        name: String,
        port: u16,
    }

    #[test]
    fn from_str_parses_toml() {
        let toml_str = r#"
            name = "recamera-app"
            port = 8080
        "#;
        let config: AppConfig = from_str(toml_str).unwrap();
        assert_eq!(config.name, "recamera-app");
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn from_str_returns_error_on_invalid_toml() {
        let result: std::result::Result<AppConfig, _> = from_str("not valid toml {{{}}}");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("failed to parse config"));
    }

    #[test]
    fn load_reads_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        write!(f, "name = \"test\"\nport = 3000").unwrap();

        let config: AppConfig = load(&path).unwrap();
        assert_eq!(config.name, "test");
        assert_eq!(config.port, 3000);
    }

    #[test]
    fn load_returns_error_on_missing_file() {
        let result: std::result::Result<AppConfig, _> = load(Path::new("/nonexistent/config.toml"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("failed to read"));
    }
}
```

- [ ] **Step 4: Add tempfile dev-dependency**

Add to `crates/recamera-config/Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p recamera-config`
Expected: 4 tests pass.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/recamera-config/
git commit -m "feat: add recamera-config crate with TOML config loading"
```

---

## Task 4: recamera-storage

**Files:**
- Create: `crates/recamera-storage/Cargo.toml`
- Create: `crates/recamera-storage/src/lib.rs`
- Modify: `Cargo.toml` (add to workspace members)

- [ ] **Step 1: Add to workspace members**

In root `Cargo.toml`, add `"crates/recamera-storage"` to `members`.

- [ ] **Step 2: Create recamera-storage Cargo.toml**

Create `crates/recamera-storage/Cargo.toml`:

```toml
[package]
name = "recamera-storage"
description = "Storage utilities for the recamera SDK"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]
recamera-core = { path = "../recamera-core" }

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Write storage module with tests**

Create `crates/recamera-storage/src/lib.rs`:

```rust
use recamera_core::{Error, FrameData, ImageFormat, Result};
use std::path::Path;

/// Information about a file in storage.
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: std::path::PathBuf,
    pub size: u64,
}

/// Information about storage capacity.
#[derive(Debug, Clone)]
pub struct StorageInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub mount_point: String,
}

/// Save raw bytes to a file.
pub fn save_file(path: &Path, data: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            Error::Storage(format!("failed to create directory {}: {e}", parent.display()))
        })?;
    }
    std::fs::write(path, data)
        .map_err(|e| Error::Storage(format!("failed to write {}: {e}", path.display())))
}

/// Save a frame as an image file.
///
/// For JPEG frames, the raw data is written directly (already encoded).
/// For RGB888 frames, the data is written as raw bytes (caller should use the `image` crate
/// for format conversion if needed).
pub fn save_image(path: &Path, frame: &FrameData) -> Result<()> {
    match frame.format {
        ImageFormat::Jpeg => save_file(path, &frame.data),
        _ => save_file(path, &frame.data),
    }
}

/// List files in a directory.
pub fn list_files(dir: &Path) -> Result<Vec<FileInfo>> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| Error::Storage(format!("failed to read directory {}: {e}", dir.display())))?;

    let mut files = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|e| Error::Storage(format!("failed to read dir entry: {e}")))?;
        let metadata = entry
            .metadata()
            .map_err(|e| Error::Storage(format!("failed to read metadata: {e}")))?;
        if metadata.is_file() {
            files.push(FileInfo {
                path: entry.path(),
                size: metadata.len(),
            });
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_file_creates_and_writes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.bin");
        save_file(&path, b"hello").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"hello");
    }

    #[test]
    fn save_file_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/b/c/test.bin");
        save_file(&path, b"nested").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"nested");
    }

    #[test]
    fn save_image_writes_jpeg_frame() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("frame.jpg");
        let frame = FrameData {
            data: vec![0xFF, 0xD8, 0xFF, 0xE0],
            width: 640,
            height: 480,
            format: ImageFormat::Jpeg,
            timestamp_ms: 0,
        };
        save_image(&path, &frame).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), vec![0xFF, 0xD8, 0xFF, 0xE0]);
    }

    #[test]
    fn list_files_returns_sorted_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("b.txt"), "b").unwrap();
        std::fs::write(dir.path().join("a.txt"), "a").unwrap();
        std::fs::create_dir(dir.path().join("subdir")).unwrap();

        let files = list_files(dir.path()).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files[0].path.ends_with("a.txt"));
        assert!(files[1].path.ends_with("b.txt"));
        assert_eq!(files[0].size, 1);
    }

    #[test]
    fn list_files_error_on_missing_dir() {
        let result = list_files(Path::new("/nonexistent/dir"));
        assert!(result.is_err());
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p recamera-storage`
Expected: 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/recamera-storage/
git commit -m "feat: add recamera-storage crate with file and image saving"
```

---

## Task 5: recamera-uart

**Files:**
- Create: `crates/recamera-uart/Cargo.toml`
- Create: `crates/recamera-uart/src/lib.rs`
- Modify: `Cargo.toml` (add to workspace members)

- [ ] **Step 1: Add to workspace members**

In root `Cargo.toml`, add `"crates/recamera-uart"` to `members`.

- [ ] **Step 2: Create recamera-uart Cargo.toml**

Create `crates/recamera-uart/Cargo.toml`:

```toml
[package]
name = "recamera-uart"
description = "UART / serial communication for the recamera SDK"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]
recamera-core = { path = "../recamera-core" }
serialport = "4"
```

- [ ] **Step 3: Write UART module with tests**

Create `crates/recamera-uart/src/lib.rs`:

```rust
use recamera_core::{Error, Result};
use std::time::Duration;

/// UART configuration.
#[derive(Debug, Clone)]
pub struct UartConfig {
    pub baud_rate: u32,
    pub data_bits: DataBits,
    pub parity: Parity,
    pub stop_bits: StopBits,
    pub timeout: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataBits {
    Five,
    Six,
    Seven,
    Eight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parity {
    None,
    Odd,
    Even,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopBits {
    One,
    Two,
}

impl Default for UartConfig {
    fn default() -> Self {
        Self {
            baud_rate: 115200,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: Duration::from_millis(1000),
        }
    }
}

impl From<DataBits> for serialport::DataBits {
    fn from(db: DataBits) -> Self {
        match db {
            DataBits::Five => serialport::DataBits::Five,
            DataBits::Six => serialport::DataBits::Six,
            DataBits::Seven => serialport::DataBits::Seven,
            DataBits::Eight => serialport::DataBits::Eight,
        }
    }
}

impl From<Parity> for serialport::Parity {
    fn from(p: Parity) -> Self {
        match p {
            Parity::None => serialport::Parity::None,
            Parity::Odd => serialport::Parity::Odd,
            Parity::Even => serialport::Parity::Even,
        }
    }
}

impl From<StopBits> for serialport::StopBits {
    fn from(sb: StopBits) -> Self {
        match sb {
            StopBits::One => serialport::StopBits::One,
            StopBits::Two => serialport::StopBits::Two,
        }
    }
}

/// UART serial port wrapper.
pub struct Uart {
    port: Box<dyn serialport::SerialPort>,
}

impl Uart {
    /// Open a serial port with the given configuration.
    pub fn open(path: &str, config: &UartConfig) -> Result<Self> {
        let port = serialport::new(path, config.baud_rate)
            .data_bits(config.data_bits.into())
            .parity(config.parity.into())
            .stop_bits(config.stop_bits.into())
            .timeout(config.timeout)
            .open()
            .map_err(|e| Error::Serial(format!("failed to open {path}: {e}")))?;

        Ok(Self { port })
    }
}

impl std::io::Read for Uart {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.port.read(buf)
    }
}

impl std::io::Write for Uart {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.port.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.port.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = UartConfig::default();
        assert_eq!(config.baud_rate, 115200);
        assert_eq!(config.data_bits, DataBits::Eight);
        assert_eq!(config.parity, Parity::None);
        assert_eq!(config.stop_bits, StopBits::One);
        assert_eq!(config.timeout, Duration::from_millis(1000));
    }

    #[test]
    fn data_bits_conversion() {
        let sp: serialport::DataBits = DataBits::Eight.into();
        assert_eq!(sp, serialport::DataBits::Eight);
    }

    #[test]
    fn parity_conversion() {
        let sp: serialport::Parity = Parity::None.into();
        assert_eq!(sp, serialport::Parity::None);
    }

    #[test]
    fn stop_bits_conversion() {
        let sp: serialport::StopBits = StopBits::One.into();
        assert_eq!(sp, serialport::StopBits::One);
    }

    #[test]
    fn open_fails_on_nonexistent_port() {
        let config = UartConfig::default();
        let result = Uart::open("/dev/nonexistent_port_12345", &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("failed to open"));
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p recamera-uart`
Expected: 5 tests pass.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/recamera-uart/
git commit -m "feat: add recamera-uart crate with serial port wrapper"
```

---

## Task 6: recamera-rs485

**Files:**
- Create: `crates/recamera-rs485/Cargo.toml`
- Create: `crates/recamera-rs485/src/lib.rs`
- Modify: `Cargo.toml` (add to workspace members)

- [ ] **Step 1: Add to workspace members**

In root `Cargo.toml`, add `"crates/recamera-rs485"` to `members`.

- [ ] **Step 2: Create recamera-rs485 Cargo.toml**

Create `crates/recamera-rs485/Cargo.toml`:

```toml
[package]
name = "recamera-rs485"
description = "RS-485 helpers for the recamera SDK"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]
recamera-core = { path = "../recamera-core" }
recamera-uart = { path = "../recamera-uart" }
```

- [ ] **Step 3: Write RS-485 module with tests**

Create `crates/recamera-rs485/src/lib.rs`:

```rust
use recamera_core::{Error, Result};
use recamera_uart::Uart;
use std::io::{Read, Write};
use std::path::Path;

/// RS-485 configuration.
#[derive(Debug, Clone)]
pub struct Rs485Config {
    /// Path to the GPIO sysfs file for the DE/RE direction pin.
    /// e.g., "/sys/class/gpio/gpio42/value"
    pub direction_gpio: Option<String>,
}

impl Default for Rs485Config {
    fn default() -> Self {
        Self {
            direction_gpio: None,
        }
    }
}

/// RS-485 wrapper around a UART port.
///
/// Handles DE/RE direction pin toggling via GPIO sysfs if configured.
pub struct Rs485 {
    uart: Uart,
    config: Rs485Config,
}

impl Rs485 {
    pub fn new(uart: Uart, config: Rs485Config) -> Self {
        Self { uart, config }
    }

    /// Set direction pin high (transmit mode).
    fn set_transmit(&self) -> Result<()> {
        if let Some(ref gpio) = self.config.direction_gpio {
            std::fs::write(Path::new(gpio), "1")
                .map_err(|e| Error::Serial(format!("failed to set TX direction: {e}")))?;
        }
        Ok(())
    }

    /// Set direction pin low (receive mode).
    fn set_receive(&self) -> Result<()> {
        if let Some(ref gpio) = self.config.direction_gpio {
            std::fs::write(Path::new(gpio), "0")
                .map_err(|e| Error::Serial(format!("failed to set RX direction: {e}")))?;
        }
        Ok(())
    }

    /// Send data over RS-485. Toggles direction pin to TX, writes, then back to RX.
    pub fn send(&mut self, data: &[u8]) -> Result<()> {
        self.set_transmit()?;
        self.uart
            .write_all(data)
            .map_err(|e| Error::Serial(format!("RS-485 write failed: {e}")))?;
        self.uart
            .flush()
            .map_err(|e| Error::Serial(format!("RS-485 flush failed: {e}")))?;
        self.set_receive()?;
        Ok(())
    }

    /// Receive data from RS-485.
    pub fn receive(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.set_receive()?;
        self.uart
            .read(buf)
            .map_err(|e| Error::Serial(format!("RS-485 read failed: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_no_gpio() {
        let config = Rs485Config::default();
        assert!(config.direction_gpio.is_none());
    }

    #[test]
    fn rs485_config_with_gpio() {
        let config = Rs485Config {
            direction_gpio: Some("/sys/class/gpio/gpio42/value".into()),
        };
        assert_eq!(
            config.direction_gpio.as_deref(),
            Some("/sys/class/gpio/gpio42/value")
        );
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p recamera-rs485`
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/recamera-rs485/
git commit -m "feat: add recamera-rs485 crate with GPIO direction control"
```

---

## Task 7: recamera-system

**Files:**
- Create: `crates/recamera-system/Cargo.toml`
- Create: `crates/recamera-system/src/lib.rs`
- Modify: `Cargo.toml` (add to workspace members)

- [ ] **Step 1: Add to workspace members**

In root `Cargo.toml`, add `"crates/recamera-system"` to `members`.

- [ ] **Step 2: Create recamera-system Cargo.toml**

Create `crates/recamera-system/Cargo.toml`:

```toml
[package]
name = "recamera-system"
description = "System and device utilities for the recamera SDK"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]
recamera-core = { path = "../recamera-core" }

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Write system module with tests**

Create `crates/recamera-system/src/lib.rs`:

```rust
use recamera_core::{Error, Result};
use std::path::{Path, PathBuf};

/// Device information.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub soc: String,
    pub os_version: String,
    pub total_memory_kb: u64,
    pub available_memory_kb: u64,
}

impl DeviceInfo {
    /// Read device info from /proc and /sys on Linux.
    pub fn get() -> Result<Self> {
        let os_version = std::fs::read_to_string("/etc/os-release")
            .unwrap_or_else(|_| "unknown".to_string());

        let meminfo =
            std::fs::read_to_string("/proc/meminfo").unwrap_or_else(|_| String::new());

        let total_memory_kb = parse_meminfo_field(&meminfo, "MemTotal");
        let available_memory_kb = parse_meminfo_field(&meminfo, "MemAvailable");

        Ok(Self {
            soc: "SG2002".to_string(),
            os_version,
            total_memory_kb,
            available_memory_kb,
        })
    }
}

fn parse_meminfo_field(meminfo: &str, field: &str) -> u64 {
    meminfo
        .lines()
        .find(|line| line.starts_with(field))
        .and_then(|line| {
            line.split_whitespace()
                .nth(1)
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or(0)
}

/// LED control via sysfs.
pub struct Led {
    brightness_path: PathBuf,
}

impl Led {
    /// Create an LED controller for the given LED name.
    /// The LED must exist at `/sys/class/leds/{name}/`.
    pub fn new(name: &str) -> Result<Self> {
        let brightness_path = Path::new("/sys/class/leds").join(name).join("brightness");
        Ok(Self { brightness_path })
    }

    /// Create an LED controller with a custom sysfs path (for testing).
    pub fn with_path(brightness_path: PathBuf) -> Self {
        Self { brightness_path }
    }

    /// Set LED brightness (typically 0 = off, 1 = on, or 0-255 for PWM).
    pub fn set_brightness(&self, value: u32) -> Result<()> {
        std::fs::write(&self.brightness_path, value.to_string()).map_err(|e| {
            Error::System(format!(
                "failed to set LED brightness at {}: {e}",
                self.brightness_path.display()
            ))
        })
    }

    /// Get current LED brightness.
    pub fn get_brightness(&self) -> Result<u32> {
        let s = std::fs::read_to_string(&self.brightness_path).map_err(|e| {
            Error::System(format!(
                "failed to read LED brightness at {}: {e}",
                self.brightness_path.display()
            ))
        })?;
        s.trim()
            .parse()
            .map_err(|e| Error::System(format!("failed to parse brightness: {e}")))
    }
}

/// Get system uptime in seconds by reading /proc/uptime.
pub fn uptime() -> Result<f64> {
    let contents = std::fs::read_to_string("/proc/uptime")
        .map_err(|e| Error::System(format!("failed to read uptime: {e}")))?;
    contents
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| Error::System("failed to parse uptime".into()))
}

/// Reboot the device.
pub fn reboot() -> Result<()> {
    std::process::Command::new("reboot")
        .status()
        .map_err(|e| Error::System(format!("failed to reboot: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_meminfo_total() {
        let meminfo = "MemTotal:        1024000 kB\nMemFree:          512000 kB\nMemAvailable:     768000 kB\n";
        assert_eq!(parse_meminfo_field(meminfo, "MemTotal"), 1024000);
        assert_eq!(parse_meminfo_field(meminfo, "MemAvailable"), 768000);
    }

    #[test]
    fn parse_meminfo_missing_field() {
        let meminfo = "MemTotal:        1024000 kB\n";
        assert_eq!(parse_meminfo_field(meminfo, "MemAvailable"), 0);
    }

    #[test]
    fn led_set_brightness_with_test_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("brightness");
        std::fs::write(&path, "0").unwrap();

        let led = Led::with_path(path.clone());
        led.set_brightness(255).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "255");
    }

    #[test]
    fn led_get_brightness_with_test_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("brightness");
        std::fs::write(&path, "128").unwrap();

        let led = Led::with_path(path);
        assert_eq!(led.get_brightness().unwrap(), 128);
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p recamera-system`
Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/recamera-system/
git commit -m "feat: add recamera-system crate with device info, LED control, and uptime"
```

---

## Task 8: recamera-cvi-sys (FFI placeholder)

**Files:**
- Create: `crates/recamera-cvi-sys/Cargo.toml`
- Create: `crates/recamera-cvi-sys/src/lib.rs`
- Create: `crates/recamera-cvi-sys/src/bindings.rs`
- Create: `crates/recamera-cvi-sys/build.rs`
- Modify: `Cargo.toml` (add to workspace members)

- [ ] **Step 1: Add to workspace members**

In root `Cargo.toml`, add `"crates/recamera-cvi-sys"` to `members`.

- [ ] **Step 2: Create recamera-cvi-sys Cargo.toml**

Create `crates/recamera-cvi-sys/Cargo.toml`:

```toml
[package]
name = "recamera-cvi-sys"
description = "Pre-generated FFI bindings for SG2002 CVI libraries (cvi_mpi, cviruntime)"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
links = "cvi"
build = "build.rs"
```

- [ ] **Step 3: Create placeholder bindings**

Create `crates/recamera-cvi-sys/src/bindings.rs`:

```rust
//! Pre-generated FFI bindings for SG2002 vendor libraries.
//!
//! These bindings were generated from the SG200X SDK headers using bindgen.
//! To regenerate, run bindgen against the SDK sysroot headers and replace this file.
//!
//! Placeholder: actual bindings will be generated once the SDK headers are available.

// --- CVI System (cvi_mpi / SYS) ---

pub const CVI_SUCCESS: i32 = 0;

// --- Video Input (VI) ---

// Placeholder for VI types and functions.
// Will contain: CVI_VI_SetDevAttr, CVI_VI_EnableDev, CVI_VI_SetChnAttr, etc.

// --- Video Processing (VPSS) ---

// Placeholder for VPSS types and functions.
// Will contain: CVI_VPSS_CreateGrp, CVI_VPSS_SetChnAttr, CVI_VPSS_SendFrame, etc.

// --- Video Encoding (VENC) ---

// Placeholder for VENC types and functions.
// Will contain: CVI_VENC_CreateChn, CVI_VENC_SendFrame, CVI_VENC_GetStream, etc.

// --- CVI Runtime (NPU inference) ---

// Placeholder for cviruntime types and functions.
// Will contain: CVI_NN_RegisterModel, CVI_NN_SetTensorWithVideoFrame, CVI_NN_Forward, etc.
```

- [ ] **Step 4: Create build.rs for link-time SDK discovery**

Create `crates/recamera-cvi-sys/build.rs`:

```rust
fn main() {
    // Only attempt to link when building for the RISC-V target.
    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.contains("riscv64") {
        // On non-RISC-V targets (e.g., macOS), skip linking.
        // This allows the crate to compile for development/testing.
        return;
    }

    let sdk_path = match std::env::var("SG200X_SDK_PATH") {
        Ok(path) => path,
        Err(_) => {
            println!(
                "cargo:warning=SG200X_SDK_PATH not set. \
                 Set it to the SDK sysroot to link CVI libraries."
            );
            return;
        }
    };

    println!("cargo:rustc-link-search=native={sdk_path}/lib");
    println!("cargo:rustc-link-lib=dylib=cvi_mpi");
    println!("cargo:rustc-link-lib=dylib=cviruntime");
    println!("cargo:rustc-link-lib=dylib=sys");
    println!("cargo:rustc-link-lib=dylib=venc");
    println!("cargo:rustc-link-lib=dylib=vpss");
    println!("cargo:rustc-link-lib=dylib=vi");
    println!("cargo:rustc-link-lib=dylib=isp");
}
```

- [ ] **Step 5: Create lib.rs**

Create `crates/recamera-cvi-sys/src/lib.rs`:

```rust
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

mod bindings;
pub use bindings::*;
```

- [ ] **Step 6: Run check (not test — no real bindings yet)**

Run: `cargo check -p recamera-cvi-sys`
Expected: Compiles with a warning about SG200X_SDK_PATH (on macOS).

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/recamera-cvi-sys/
git commit -m "feat: add recamera-cvi-sys crate with FFI placeholder and build-time SDK discovery"
```

---

## Task 9: recamera-camera

**Files:**
- Create: `crates/recamera-camera/Cargo.toml`
- Create: `crates/recamera-camera/src/lib.rs`
- Modify: `Cargo.toml` (add to workspace members)

- [ ] **Step 1: Add to workspace members**

In root `Cargo.toml`, add `"crates/recamera-camera"` to `members`.

- [ ] **Step 2: Create recamera-camera Cargo.toml**

Create `crates/recamera-camera/Cargo.toml`:

```toml
[package]
name = "recamera-camera"
description = "Camera capture for the recamera SDK"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]
recamera-core = { path = "../recamera-core" }
recamera-cvi-sys = { path = "../recamera-cvi-sys" }
```

- [ ] **Step 3: Write camera module with config and types**

Create `crates/recamera-camera/src/lib.rs`:

```rust
use recamera_core::{Error, FrameData, ImageFormat, Resolution, Result};

/// Camera channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// Channel 0: RAW / RGB888
    Raw,
    /// Channel 1: JPEG
    Jpeg,
    /// Channel 2: H.264
    H264,
}

/// Camera configuration.
#[derive(Debug, Clone)]
pub struct CameraConfig {
    pub resolution: Resolution,
    pub fps: u32,
    pub channel: Channel,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            resolution: Resolution::new(1920, 1080),
            fps: 30,
            channel: Channel::Jpeg,
        }
    }
}

/// A captured frame. Implements Drop to release the underlying buffer.
pub struct Frame {
    pub data: FrameData,
}

impl Frame {
    pub fn width(&self) -> u32 {
        self.data.width
    }

    pub fn height(&self) -> u32 {
        self.data.height
    }

    pub fn format(&self) -> ImageFormat {
        self.data.format
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data.data
    }

    pub fn timestamp_ms(&self) -> u64 {
        self.data.timestamp_ms
    }
}

/// Camera handle for capturing frames from the reCamera hardware.
///
/// Wraps the CVI MPI video pipeline: Sensor -> VI -> ISP -> VPSS -> VENC.
///
/// NOTE: Actual hardware initialization requires the CVI vendor libraries.
/// On non-RISC-V targets, the camera will return a "not available" error.
pub struct Camera {
    config: CameraConfig,
    streaming: bool,
}

impl Camera {
    /// Initialize the camera with the given configuration.
    ///
    /// On the reCamera device, this sets up the VI/VPSS/VENC pipeline.
    /// On other platforms, this returns an error.
    pub fn new(config: CameraConfig) -> Result<Self> {
        #[cfg(not(target_arch = "riscv64"))]
        {
            let _ = &config;
            return Err(Error::Camera(
                "camera hardware not available on this platform".into(),
            ));
        }

        #[cfg(target_arch = "riscv64")]
        {
            // TODO: Initialize CVI MPI pipeline via recamera_cvi_sys
            // CVI_SYS_Init() -> CVI_VI_SetDevAttr() -> CVI_VPSS_CreateGrp() -> etc.
            Ok(Self {
                config,
                streaming: false,
            })
        }
    }

    /// Capture a single frame.
    pub fn capture(&self) -> Result<Frame> {
        if !self.streaming {
            return Err(Error::Camera("camera is not streaming".into()));
        }

        #[cfg(not(target_arch = "riscv64"))]
        {
            return Err(Error::Camera(
                "camera hardware not available on this platform".into(),
            ));
        }

        #[cfg(target_arch = "riscv64")]
        {
            // TODO: CVI_VPSS_GetChnFrame / CVI_VENC_GetStream
            todo!("implement frame capture via CVI MPI")
        }
    }

    /// Start continuous capture.
    pub fn start_stream(&mut self) -> Result<()> {
        #[cfg(not(target_arch = "riscv64"))]
        {
            return Err(Error::Camera(
                "camera hardware not available on this platform".into(),
            ));
        }

        #[cfg(target_arch = "riscv64")]
        {
            // TODO: Start VI/VPSS/VENC pipeline
            self.streaming = true;
            Ok(())
        }
    }

    /// Stop capture.
    pub fn stop_stream(&mut self) -> Result<()> {
        #[cfg(not(target_arch = "riscv64"))]
        {
            return Err(Error::Camera(
                "camera hardware not available on this platform".into(),
            ));
        }

        #[cfg(target_arch = "riscv64")]
        {
            // TODO: Stop VI/VPSS/VENC pipeline
            self.streaming = false;
            Ok(())
        }
    }

    pub fn config(&self) -> &CameraConfig {
        &self.config
    }

    pub fn is_streaming(&self) -> bool {
        self.streaming
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_camera_config() {
        let config = CameraConfig::default();
        assert_eq!(config.resolution, Resolution::new(1920, 1080));
        assert_eq!(config.fps, 30);
        assert_eq!(config.channel, Channel::Jpeg);
    }

    #[test]
    fn frame_accessors() {
        let frame = Frame {
            data: FrameData {
                data: vec![1, 2, 3],
                width: 640,
                height: 480,
                format: ImageFormat::Jpeg,
                timestamp_ms: 99,
            },
        };
        assert_eq!(frame.width(), 640);
        assert_eq!(frame.height(), 480);
        assert_eq!(frame.format(), ImageFormat::Jpeg);
        assert_eq!(frame.as_bytes(), &[1, 2, 3]);
        assert_eq!(frame.timestamp_ms(), 99);
    }

    #[test]
    #[cfg(not(target_arch = "riscv64"))]
    fn camera_new_fails_on_non_riscv() {
        let result = Camera::new(CameraConfig::default());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not available on this platform"));
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p recamera-camera`
Expected: 3 tests pass (on macOS).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/recamera-camera/
git commit -m "feat: add recamera-camera crate with config, frame types, and platform-gated init"
```

---

## Task 10: recamera-infer

**Files:**
- Create: `crates/recamera-infer/Cargo.toml`
- Create: `crates/recamera-infer/src/lib.rs`
- Modify: `Cargo.toml` (add to workspace members)

- [ ] **Step 1: Add to workspace members**

In root `Cargo.toml`, add `"crates/recamera-infer"` to `members`.

- [ ] **Step 2: Create recamera-infer Cargo.toml**

Create `crates/recamera-infer/Cargo.toml`:

```toml
[package]
name = "recamera-infer"
description = "Local inference engine for the recamera SDK (.cvimodel)"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]
recamera-core = { path = "../recamera-core" }
recamera-cvi-sys = { path = "../recamera-cvi-sys" }
```

- [ ] **Step 3: Write inference module with types and tests**

Create `crates/recamera-infer/src/lib.rs`:

```rust
use recamera_core::{Error, FrameData, Result};
use std::path::{Path, PathBuf};

/// Tensor shape description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TensorShape {
    pub dims: Vec<usize>,
}

impl TensorShape {
    pub fn new(dims: Vec<usize>) -> Self {
        Self { dims }
    }

    pub fn total_elements(&self) -> usize {
        self.dims.iter().product()
    }
}

/// Model metadata.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub path: PathBuf,
    pub input_shape: TensorShape,
    pub output_shapes: Vec<TensorShape>,
}

/// A single detection result.
#[derive(Debug, Clone)]
pub struct Detection {
    /// Center X (0.0 - 1.0, normalized)
    pub x: f32,
    /// Center Y (0.0 - 1.0, normalized)
    pub y: f32,
    /// Width (0.0 - 1.0, normalized)
    pub w: f32,
    /// Height (0.0 - 1.0, normalized)
    pub h: f32,
    /// Class ID
    pub class_id: u32,
    /// Confidence score (0.0 - 1.0)
    pub score: f32,
}

/// Inference output.
#[derive(Debug, Clone)]
pub enum Output {
    Detections(Vec<Detection>),
    Classification { class_id: u32, score: f32 },
    Raw(Vec<Vec<f32>>),
}

/// CVI NPU inference engine.
///
/// Manages the CVI runtime and provides model loading.
/// On non-RISC-V targets, returns a "not available" error.
pub struct Engine {
    _private: (),
}

impl Engine {
    /// Initialize the CVI NPU runtime.
    pub fn new() -> Result<Self> {
        #[cfg(not(target_arch = "riscv64"))]
        {
            return Err(Error::Inference(
                "CVI NPU not available on this platform".into(),
            ));
        }

        #[cfg(target_arch = "riscv64")]
        {
            // TODO: Initialize CVI runtime via recamera_cvi_sys
            Ok(Self { _private: () })
        }
    }

    /// Load a `.cvimodel` file.
    ///
    /// The model must be pre-converted from ONNX using Sophgo's offline toolchain.
    pub fn load_model(&self, path: &Path) -> Result<Model> {
        if !path.exists() {
            return Err(Error::Inference(format!(
                "model file not found: {}",
                path.display()
            )));
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if ext != "cvimodel" {
            return Err(Error::Inference(format!(
                "expected .cvimodel file, got .{ext}"
            )));
        }

        #[cfg(not(target_arch = "riscv64"))]
        {
            return Err(Error::Inference(
                "CVI NPU not available on this platform".into(),
            ));
        }

        #[cfg(target_arch = "riscv64")]
        {
            // TODO: CVI_NN_RegisterModel, parse input/output tensors
            todo!("implement model loading via CVI runtime")
        }
    }
}

/// A loaded inference model.
pub struct Model {
    pub info: ModelInfo,
    _private: (),
}

impl Model {
    /// Run inference on a frame.
    pub fn run(&self, _input: &FrameData) -> Result<Output> {
        #[cfg(not(target_arch = "riscv64"))]
        {
            return Err(Error::Inference(
                "CVI NPU not available on this platform".into(),
            ));
        }

        #[cfg(target_arch = "riscv64")]
        {
            // TODO: CVI_NN_SetTensorWithVideoFrame -> CVI_NN_Forward -> parse output
            todo!("implement inference via CVI runtime")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tensor_shape_total_elements() {
        let shape = TensorShape::new(vec![1, 3, 224, 224]);
        assert_eq!(shape.total_elements(), 1 * 3 * 224 * 224);
    }

    #[test]
    fn tensor_shape_empty() {
        let shape = TensorShape::new(vec![]);
        assert_eq!(shape.total_elements(), 1); // product of empty = 1
    }

    #[test]
    fn detection_fields() {
        let det = Detection {
            x: 0.5,
            y: 0.5,
            w: 0.1,
            h: 0.2,
            class_id: 0,
            score: 0.95,
        };
        assert_eq!(det.class_id, 0);
        assert!(det.score > 0.9);
    }

    #[test]
    fn output_detections_variant() {
        let output = Output::Detections(vec![Detection {
            x: 0.5,
            y: 0.5,
            w: 0.1,
            h: 0.2,
            class_id: 1,
            score: 0.8,
        }]);
        match output {
            Output::Detections(dets) => assert_eq!(dets.len(), 1),
            _ => panic!("expected Detections variant"),
        }
    }

    #[test]
    #[cfg(not(target_arch = "riscv64"))]
    fn engine_new_fails_on_non_riscv() {
        let result = Engine::new();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not available on this platform"));
    }

    #[test]
    fn load_model_rejects_wrong_extension() {
        // Create a temp file with wrong extension
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("model.onnx");
        std::fs::write(&path, b"fake").unwrap();

        // We can't call engine.load_model on non-riscv, so test the validation logic directly.
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        assert_ne!(ext, "cvimodel");
    }
}
```

- [ ] **Step 4: Add tempfile dev-dependency**

Add to `crates/recamera-infer/Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p recamera-infer`
Expected: 6 tests pass.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/recamera-infer/
git commit -m "feat: add recamera-infer crate with engine, model, and detection types"
```

---

## Task 11: Facade Crate (recamera)

**Files:**
- Create: `crates/recamera/Cargo.toml`
- Create: `crates/recamera/src/lib.rs`
- Modify: `Cargo.toml` (add to workspace members)

- [ ] **Step 1: Add to workspace members**

In root `Cargo.toml`, add `"crates/recamera"` to `members`.

- [ ] **Step 2: Create recamera facade Cargo.toml**

Create `crates/recamera/Cargo.toml`:

```toml
[package]
name = "recamera"
description = "Rust SDK for Seeed reCamera — camera capture, inference, serial, storage, and more"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]
recamera-core = { path = "../recamera-core" }
recamera-camera = { path = "../recamera-camera", optional = true }
recamera-infer = { path = "../recamera-infer", optional = true }
recamera-uart = { path = "../recamera-uart", optional = true }
recamera-rs485 = { path = "../recamera-rs485", optional = true }
recamera-storage = { path = "../recamera-storage", optional = true }
recamera-logging = { path = "../recamera-logging", optional = true }
recamera-config = { path = "../recamera-config", optional = true }
recamera-system = { path = "../recamera-system", optional = true }

[features]
default = ["logging", "config", "system"]
camera = ["dep:recamera-camera"]
infer = ["dep:recamera-infer"]
uart = ["dep:recamera-uart"]
rs485 = ["uart", "dep:recamera-rs485"]
storage = ["dep:recamera-storage"]
logging = ["dep:recamera-logging"]
config = ["dep:recamera-config"]
system = ["dep:recamera-system"]
full = ["camera", "infer", "uart", "rs485", "storage", "logging", "config", "system"]
```

- [ ] **Step 3: Create facade lib.rs**

Create `crates/recamera/src/lib.rs`:

```rust
//! # recamera
//!
//! Rust SDK for Seeed reCamera (SG2002).
//!
//! Enable features for the modules you need:
//!
//! ```toml
//! [dependencies]
//! recamera = { version = "0.1", features = ["camera", "uart", "storage"] }
//! ```

pub use recamera_core as core;
pub use recamera_core::{Error, ImageFormat, Resolution, Result};

#[cfg(feature = "camera")]
pub use recamera_camera as camera;

#[cfg(feature = "infer")]
pub use recamera_infer as infer;

#[cfg(feature = "uart")]
pub use recamera_uart as uart;

#[cfg(feature = "rs485")]
pub use recamera_rs485 as rs485;

#[cfg(feature = "storage")]
pub use recamera_storage as storage;

#[cfg(feature = "logging")]
pub use recamera_logging as logging;

#[cfg(feature = "config")]
pub use recamera_config as config;

#[cfg(feature = "system")]
pub use recamera_system as system;
```

- [ ] **Step 4: Verify it compiles with default features**

Run: `cargo check -p recamera`
Expected: Compiles (default features: logging, config, system).

- [ ] **Step 5: Verify it compiles with all features**

Run: `cargo check -p recamera --features full`
Expected: Compiles with all features enabled.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/recamera/
git commit -m "feat: add recamera facade crate with feature-gated re-exports"
```

---

## Task 12: Cross-Compilation Config

**Files:**
- Create: `.cargo/config.toml`

- [ ] **Step 1: Create .cargo/config.toml**

Create `.cargo/config.toml`:

```toml
# Cross-compilation for reCamera (SG2002 RISC-V)
#
# Prerequisites:
# 1. Install the RISC-V target: rustup target add riscv64gc-unknown-linux-musl
# 2. Set SG200X_SDK_PATH to the SDK sysroot (for camera/infer features)
# 3. Ensure a RISC-V cross-linker is available (e.g., from the SDK toolchain)
#
# To build for reCamera:
#   cargo build --target riscv64gc-unknown-linux-musl --release
#
# To build only pure-Rust crates (no SDK needed):
#   cargo build --target riscv64gc-unknown-linux-musl --release -p recamera --no-default-features --features "logging,config"

[target.riscv64gc-unknown-linux-musl]
# Uncomment and set to your cross-compiler path:
# linker = "/path/to/riscv64-unknown-linux-musl-gcc"
```

- [ ] **Step 2: Commit**

```bash
git add .cargo/config.toml
git commit -m "feat: add cross-compilation config for riscv64gc-unknown-linux-musl"
```

---

## Task 13: Workspace-Wide Test + Final Verify

- [ ] **Step 1: Run all workspace tests**

Run: `cargo test --workspace`
Expected: All tests across all crates pass.

- [ ] **Step 2: Run cargo clippy**

Run: `cargo clippy --workspace --all-features -- -D warnings`
Expected: No warnings.

- [ ] **Step 3: Fix any clippy issues found**

Address any lint warnings and re-run clippy until clean.

- [ ] **Step 4: Run cargo fmt check**

Run: `cargo fmt --all -- --check`
Expected: All code is formatted.

- [ ] **Step 5: Commit any fixes**

```bash
git add -A
git commit -m "chore: fix clippy warnings and formatting"
```
