# recamera-rs

A Rust SDK for [Seeed Studio reCamera](https://www.seeedstudio.com/recamera) -- camera capture, local inference, serial I/O, storage, and system utilities for edge vision applications on the SG2002 SoC.

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

| Field               | Type    | Default  | Description                                            |
| ------------------- | ------- | -------- | ------------------------------------------------------ |
| `fps`               | integer | `30`     | Target frame rate in frames per second                 |
| `channel`           | string  | `"jpeg"` | Video channel: `"raw"` (RGB888), `"jpeg"`, or `"h264"` |
| `resolution.width`  | integer | `1920`   | Capture width in pixels                                |
| `resolution.height` | integer | `1080`   | Capture height in pixels                               |

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

Requires the [reCamera-OS SDK](https://github.com/Seeed-Studio/reCamera-OS/releases) for cross-compilation (see [Cross-Compiling](#cross-compiling-for-recamera) below).

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

| Feature   | Description                             | Default |
| --------- | --------------------------------------- | ------- |
| `camera`  | Camera capture and frame handling       | No      |
| `infer`   | Local inference engine (.cvimodel)      | No      |
| `uart`    | UART / serial communication             | No      |
| `rs485`   | RS-485 helpers (enables `uart`)         | No      |
| `storage` | Image and file storage utilities        | No      |
| `logging` | Logging utilities                       | Yes     |
| `config`  | Configuration loading and validation    | Yes     |
| `system`  | System and device information utilities | Yes     |
| `serde`   | Serialization support for config types  | No      |
| `full`    | Enables all features                    | No      |

## Versioning

This project follows [Semantic Versioning](https://semver.org/). While the SDK is pre-1.0, minor version bumps may include breaking changes.

## Crates

| Crate              | Description                                           |
| ------------------ | ----------------------------------------------------- |
| `recamera`         | Facade -- re-exports subcrates based on feature flags |
| `recamera-core`    | Shared types, errors, and traits                      |
| `recamera-camera`  | Camera capture via CVI MPI (VI/VPSS/VENC)             |
| `recamera-infer`   | NPU inference for .cvimodel files                     |
| `recamera-cvi-sys` | FFI bindings for SG2002 CVI libs (compile-time linked) |
| `recamera-uart`    | UART / serial communication                           |
| `recamera-rs485`   | RS-485 helpers built on UART                          |
| `recamera-storage` | Image and file storage utilities                      |
| `recamera-logging` | Logging utilities (tracing)                           |
| `recamera-config`  | TOML configuration loading (serde)                    |
| `recamera-system`  | Device info, LED control, system utilities            |

## How It Works

The vendor C libraries (camera, video, NPU inference) are linked at **compile time** against the SDK copies in `sdk/sg2002_recamera_emmc/cvi_mpi/lib/`. At runtime, the device's dynamic linker loads the shared libraries and resolves transitive dependencies (e.g. `libatomic.so`) automatically.

`recamera-cvi-sys` provides:

- Type definitions, structs, enums, and constants generated from the SDK headers
- `extern "C"` declarations for all vendor functions (linked via `build.rs`)

The higher-level crates (`recamera-camera`, `recamera-infer`) call these functions directly through safe Rust wrappers.

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

### Step 3: Download the reCamera-OS SDK

Download the SDK matching your device's firmware version from [reCamera-OS releases](https://github.com/Seeed-Studio/reCamera-OS/releases):

```bash
wget https://github.com/Seeed-Studio/reCamera-OS/releases/download/0.2.1/sg2002_reCamera_0.2.1_emmc_sdk.tar.gz
tar xzf sg2002_reCamera_0.2.1_emmc_sdk.tar.gz
```

### Step 4: Configure Cargo in your project

In your own project (not the SDK), create `.cargo/config.toml`:

```toml
[build]
target = "riscv64gc-unknown-linux-musl"

[target.riscv64gc-unknown-linux-musl]
linker = "riscv64-unknown-linux-musl-gcc"
rustflags = ["-C", "link-arg=-Wl,--allow-shlib-undefined"]
```

The `--allow-shlib-undefined` flag tells the linker not to resolve transitive dependencies of the vendor shared libraries — those are resolved at runtime on the device. This is the default on glibc-based linkers but must be set explicitly with the musl toolchain.

### Step 5: Build your project

Set `CVI_MPI_LIB_DIR` to point at the SDK's vendor libraries, then build:

```bash
export CVI_MPI_LIB_DIR=/path/to/sg2002_recamera_emmc/cvi_mpi/lib
cargo build --release
```

This compiles your application (which pulls in `recamera-rs` as a dependency) for the reCamera's RISC-V target.

Output binary: `target/riscv64gc-unknown-linux-musl/release/<your-crate-name>`

### Step 6: Deploy to device

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
