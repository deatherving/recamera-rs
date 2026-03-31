# recamera-rs

A Rust SDK for [Seeed reCamera](https://wiki.seeedstudio.com/recamera/) -- camera capture, local inference, serial I/O, storage, and system utilities for edge vision applications on the SG2002 SoC.

> This is a community project and is not affiliated with or officially maintained by Seeed Studio.

## Quick Start

Add `recamera` to your project:

```toml
[dependencies]
recamera = { git = "https://github.com/deatherving/recamera-rs", features = ["camera", "config", "serde"] }
```

Create a config file in your project (e.g., `config/camera.toml`):

```toml
# config/camera.toml
fps = 15
channel = "jpeg"

[resolution]
width = 1280
height = 720
```

All fields are optional and fall back to defaults if omitted:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `fps` | integer | `30` | Target frame rate in frames per second |
| `channel` | string | `"jpeg"` | Video channel: `"raw"` (RGB888), `"jpeg"`, or `"h264"` |
| `resolution.width` | integer | `1920` | Capture width in pixels |
| `resolution.height` | integer | `1080` | Capture height in pixels |

Capture a frame:

```rust
use recamera::camera::{Camera, CameraConfig};
use std::path::Path;

let config: CameraConfig = recamera::config::load(Path::new("config/camera.toml"))?;
let mut camera = Camera::new(config)?;
camera.start_stream()?;
let frame = camera.capture()?;
println!("Captured {}x{} frame", frame.width(), frame.height());
```

No SDK download required. The vendor libraries are loaded at runtime on the reCamera device.

## Camera + Inference Pipeline

Capture a frame and run a .cvimodel on the NPU:

```toml
[dependencies]
recamera = { git = "https://github.com/deatherving/recamera-rs", features = ["camera", "infer", "config", "serde"] }
```

```rust
use recamera::camera::{Camera, CameraConfig};
use recamera::infer::{Engine, Output};
use std::path::Path;

let config: CameraConfig = recamera::config::load(Path::new("camera.toml"))?;
let mut camera = Camera::new(config)?;
camera.start_stream()?;
let frame = camera.capture()?;

let engine = Engine::new()?;
let model = engine.load_model(Path::new("/userdata/models/yolo.cvimodel"))?;
let output = model.run(&frame.data)?;

match output {
    Output::Raw(tensors) => {
        println!("Model returned {} output tensors", tensors.len());
    }
    _ => {}
}
```

The `.cvimodel` file must be pre-converted from ONNX using Sophgo's offline toolchain.

## Features

| Feature   | Description                              | Default |
|-----------|------------------------------------------|---------|
| `camera`  | Camera capture and frame handling        | No      |
| `infer`   | Local inference engine (.cvimodel)       | No      |
| `uart`    | UART / serial communication              | No      |
| `rs485`   | RS-485 helpers (enables `uart`)          | No      |
| `storage` | Image and file storage utilities         | No      |
| `logging` | Logging utilities                        | Yes     |
| `config`  | Configuration loading and validation     | Yes     |
| `system`  | System and device information utilities  | Yes     |
| `serde`   | Serialization support for config types   | No      |
| `full`    | Enables all features                     | No      |

## Versioning

This project follows [Semantic Versioning](https://semver.org/). While the SDK is pre-1.0, minor version bumps may include breaking changes.

## Crates

| Crate              | Description                                            |
|--------------------|--------------------------------------------------------|
| `recamera`         | Facade -- re-exports subcrates based on feature flags  |
| `recamera-core`    | Shared types, errors, and traits                       |
| `recamera-camera`  | Camera capture via CVI MPI (VI/VPSS/VENC)              |
| `recamera-infer`   | NPU inference for .cvimodel files                      |
| `recamera-cvi-sys` | FFI bindings and runtime loader for SG2002 CVI libs    |
| `recamera-uart`    | UART / serial communication                            |
| `recamera-rs485`   | RS-485 helpers built on UART                           |
| `recamera-storage` | Image and file storage utilities                       |
| `recamera-logging` | Logging utilities (tracing)                            |
| `recamera-config`  | TOML configuration loading (serde)                     |
| `recamera-system`  | Device info, LED control, system utilities             |

## How It Works

The vendor C libraries (camera, video, NPU inference) are loaded at **runtime** on the reCamera device using `dlopen`. No compile-time linking or SDK download is needed to build your application.

`recamera-cvi-sys` provides:
- Type definitions, structs, enums, and constants generated from the SDK headers
- A runtime loader (`CviLibs`) that finds and loads the vendor `.so` libraries on the device

The higher-level crates (`recamera-camera`, `recamera-infer`) wrap the loader with safe Rust APIs.

## Cross-Compiling for reCamera

The reCamera uses a RISC-V SG2002 SoC. Cross-compilation must be done on a **Linux machine** (Ubuntu 22.04+, Amazon Linux 2023, or similar). macOS and Windows may not be supported as build hosts.

### Step 1: Install Rust and the RISC-V target

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
rustup target add riscv64gc-unknown-linux-musl
```

### Step 2: Install the RISC-V cross-compilation toolchain

The [Sophgo host-tools](https://github.com/sophgo/host-tools) GCC (10.2.0) (referenced in the [reCamera C/C++ development wiki](https://wiki.seeedstudio.com/recamera_develop_with_c_cpp/)) shipped with the reCamera SDK is too old for Rust 1.85+ — its binutils cannot handle the RISC-V ISA extensions that LLVM 19 emits. Use the [riscv-collab toolchain](https://github.com/riscv-collab/riscv-gnu-toolchain/releases) (binutils 2.39+) instead.

```bash
wget https://github.com/riscv-collab/riscv-gnu-toolchain/releases/download/2026.03.28/riscv64-musl-ubuntu-22.04-gcc.tar.xz
mkdir -p ~/riscv-toolchain && tar xf riscv64-musl-ubuntu-22.04-gcc.tar.xz -C ~/riscv-toolchain
```

Add the toolchain to your `PATH` (add this to `~/.bashrc` to make it permanent):

```bash
export PATH=$HOME/riscv-toolchain/riscv/bin:$PATH
```

### Step 3: Configure Cargo in your project

In your own project (not the SDK), create `.cargo/config.toml` to set the default target and linker:

```toml
[build]
target = "riscv64gc-unknown-linux-musl"

[target.riscv64gc-unknown-linux-musl]
linker = "riscv64-unknown-linux-musl-gcc"
```

### Step 4: Build your project

From your project directory, run:

```bash
cargo build --release
```

This compiles your application (which pulls in `recamera-rs` as a dependency) for the reCamera's RISC-V target. The SDK itself does not need to be built separately — Cargo fetches and compiles it automatically.

Output binary: `target/riscv64gc-unknown-linux-musl/release/<your-crate-name>`

### Step 5: Deploy to device

```bash
scp target/riscv64gc-unknown-linux-musl/release/<binary> recamera@<device-ip>:/home/recamera/
```

## License

Licensed under either of:

- [MIT license](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.

## Contributing

Contributions, issues, and suggestions are welcome.
