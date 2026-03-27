# recamera-rs SDK Design Spec

**Date:** 2026-03-27
**Status:** Approved

## Overview

A Rust SDK for Seeed reCamera (SG2002 RISC-V SoC), providing ergonomic access to camera capture, local inference, UART/RS-485, storage, logging, configuration, and system utilities.

The SDK wraps vendor C libraries (cvi_mpi, cviruntime) via FFI for hardware access, and uses pure Rust for everything else. It is designed to cross-compile on macOS and deploy to reCamera.

## Scope

**In scope:**
- Camera capture
- Local inference (`.cvimodel` — pre-converted from ONNX offline)
- UART / serial communication
- RS-485 support helpers
- Storage utilities
- Logging
- Configuration management
- System and device utilities

**Out of scope:**
- HTTP server, MQTT, RTSP
- ONNX model conversion (done offline with Sophgo toolchain)
- Application-specific logic
- End-to-end model training

## Architecture: Facade + Workspace

Monorepo Cargo workspace with individual crates, plus a top-level `recamera` facade crate that re-exports everything via feature flags.

Users add one dependency and enable features for what they need:

```toml
[dependencies]
recamera = { version = "0.1", features = ["camera", "uart"] }
```

Individual crates are also usable directly for fine-grained control.

## Project Structure

```
recamera-rs/
  Cargo.toml              # Workspace root
  crates/
    recamera/             # Facade crate — re-exports with feature flags
    recamera-core/        # Shared Error type, traits, common types
    recamera-camera/      # Camera capture (wraps cvi-sys)
    recamera-infer/       # Inference engine (wraps cvi-sys)
    recamera-uart/        # UART / serial
    recamera-rs485/       # RS-485 helpers (depends on uart)
    recamera-storage/     # Image/file storage utilities
    recamera-logging/     # Logging setup (thin wrapper around tracing)
    recamera-config/      # Config loading/validation (serde-based)
    recamera-system/      # Device info, LED control, system utilities
    recamera-cvi-sys/     # Pre-generated C FFI bindings for SG2002
  examples/               # End-to-end example binaries
```

## Facade Crate Features

```toml
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

## Crate Dependency Graph

```
recamera (facade)
  ├── recamera-core (always)
  ├── recamera-camera ──► recamera-cvi-sys
  ├── recamera-infer ──► recamera-cvi-sys
  ├── recamera-uart
  ├── recamera-rs485 ──► recamera-uart
  ├── recamera-storage
  ├── recamera-logging
  ├── recamera-config
  └── recamera-system
```

All crates depend on `recamera-core` for shared error types and traits.

## Crate API Designs

### `recamera-core`

- `Error` enum — unified error type across all crates (IO, Config, Camera, Inference, Serial variants)
- `Result<T>` type alias
- Common types: `ImageFormat` (RGB888, JPEG, H264, NV21), `Resolution`, `FrameData`

### `recamera-camera` (feature: `camera`)

- `Camera::new(config) -> Result<Camera>` — initialize camera with sensor/channel config
- `Camera::capture(&self, format: ImageFormat) -> Result<Frame>` — grab a single frame
- `Camera::start_stream(&self) -> Result<FrameStream>` — continuous capture, returns an iterator/async stream
- `Camera::stop_stream(&self)` — stop capture
- `Frame` — holds image data, dimensions, timestamp, format. Implements `Drop` to release the underlying buffer.

Wraps the CVI MPI video pipeline: Sensor → VI → ISP → VPSS → VENC. Three hardware channels (CH0=RAW/RGB888, CH1=JPEG, CH2=H.264) can run simultaneously.

### `recamera-infer` (feature: `infer`)

- `Engine::new() -> Result<Engine>` — init CVI NPU runtime
- `Engine::load_model(path) -> Result<Model>` — load a `.cvimodel` file (pre-converted from ONNX using Sophgo's offline toolchain)
- `Model::run(&self, input: &Frame) -> Result<Output>` — run inference on the NPU
- `Output` — detection results (bounding boxes, class IDs, scores), classification results, etc.
- `ModelInfo` — input/output tensor shapes, model type

The SDK does NOT handle ONNX → cvimodel conversion. That is an offline step using Sophgo's model conversion tools.

### `recamera-uart` (feature: `uart`)

- `Uart::open(port, config) -> Result<Uart>` — open serial port with baud rate, parity, etc.
- `Uart::read(&self, buf) -> Result<usize>`
- `Uart::write(&self, data) -> Result<usize>`
- Implements `std::io::Read` + `Write` for composability
- Built on `serialport` crate

### `recamera-rs485` (feature: `rs485`)

- `Rs485::new(uart, config) -> Result<Rs485>` — wraps a `Uart` with RS-485 direction control
- `Rs485::send(&self, data) -> Result<()>`
- `Rs485::receive(&self, buf) -> Result<usize>`
- Handles DE/RE pin toggling via GPIO

### `recamera-storage` (feature: `storage`)

- `save_image(path, frame) -> Result<()>` — save frame as JPEG/PNG
- `save_file(path, data) -> Result<()>`
- `list_files(dir) -> Result<Vec<FileInfo>>`
- `StorageInfo` — available space, mount point

### `recamera-logging` (feature: `logging`)

- `init(config) -> Result<()>` — set up `tracing` subscriber with file + stdout output
- `LogConfig` — level, output path, rotation settings

### `recamera-config` (feature: `config`)

- `Config::load(path) -> Result<Config>` — load TOML/JSON config file
- `Config::get<T>(key) -> Result<T>` — typed access
- `Config::validate() -> Result<()>`
- Serde-based, user defines their own config structs

### `recamera-system` (feature: `system`)

- `DeviceInfo::get() -> Result<DeviceInfo>` — SoC, memory, OS version
- `Led::new(name) -> Result<Led>` — control via sysfs (`/sys/class/leds/`)
- `Led::set_brightness(&self, value)`
- `system::reboot()`, `system::uptime()`

## FFI Strategy

- **Pre-generated bindings** checked into `recamera-cvi-sys/`
- Generated once from SG200X SDK headers using `bindgen`, committed to repo
- No build-time dependency on SDK headers — the crate compiles anywhere
- `SG200X_SDK_PATH` env var used only at link time when building final binaries with `camera` or `infer` features
- Pure-Rust crates compile and test on macOS without the SDK

## Cross-Compilation

- **Host:** macOS (Apple Silicon)
- **Target:** `riscv64gc-unknown-linux-musl`
- Toolchain configured via `.cargo/config.toml` with linker pointing to RISC-V cross-compiler
- `SG200X_SDK_PATH` env var points to the sysroot (only needed for `camera`/`infer` features)
- Pure-Rust crates compile and test on macOS without the SDK

## Hardware Context

- **SoC:** Sophgo SG2002 (T-Head C906 RISC-V core, 1 TOPS NPU)
- **Camera sensors:** OV5647, GC2053, IMX335, SC130GS
- **OS:** Linux (musl libc, Buildroot-based)
- **Video pipeline:** Sensor → VI → ISP → VPSS → VENC

## Testing Strategy

- **Pure-Rust crates:** unit tests run on macOS
- **FFI crates:** compile-checked on macOS (behind feature flags), integration-tested on device
- **Examples:** build for RISC-V, manual test on reCamera

## Implementation Order

1. Workspace + `recamera-core` (errors, traits, types)
2. Pure-Rust crates (uart, storage, logging, config, system)
3. FFI `-sys` crate with pre-generated bindings
4. `recamera-camera` + `recamera-infer` wrapping the FFI
5. Facade crate (`recamera`) with feature flags
6. End-to-end examples: camera → inference → output
