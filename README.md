# recamera-rs

A Rust SDK for Seeed reCamera -- camera capture, local inference, serial I/O, storage, and system utilities for edge vision applications.

> **Disclaimer:** This is a community project and is not affiliated with or officially maintained by Seeed Studio.

## Usage

Add `recamera` to your `Cargo.toml` with the features you need:

```toml
[dependencies]
recamera = { version = "0.1", features = ["uart", "storage"] }
```

To enable everything:

```toml
[dependencies]
recamera = { version = "0.1", features = ["full"] }
```

## Feature Flags

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
| `full`    | Enables all features                     | No      |

## Crate Structure

| Crate                | Description                                                        |
|----------------------|--------------------------------------------------------------------|
| `recamera`           | Facade crate -- re-exports subcrates based on feature flags        |
| `recamera-core`      | Shared types, errors, and traits                                   |
| `recamera-camera`    | Camera capture and frame handling                                  |
| `recamera-infer`     | Local inference engine for .cvimodel files                         |
| `recamera-cvi-sys`   | Pre-generated FFI bindings for SG2002 CVI libraries                |
| `recamera-uart`      | UART / serial communication                                       |
| `recamera-rs485`     | RS-485 helpers built on top of UART                                |
| `recamera-storage`   | Image and file storage utilities                                   |
| `recamera-logging`   | Logging utilities                                                  |
| `recamera-config`    | Configuration loading and validation                               |
| `recamera-system`    | System and device information utilities                            |

## Status

This project is at an early stage. The API is expected to change as the design stabilizes.

- Pure-Rust crates (`core`, `uart`, `rs485`, `storage`, `logging`, `config`, `system`) are functional.
- FFI crates (`cvi-sys`) include pre-generated bindings for the CVI MPI camera/video libraries (263 functions).
- NPU inference bindings are not yet available (cviruntime headers not included in current SDK release).
- `camera` and `infer` crate implementations are stubbed, pending wiring to the FFI bindings.

## Getting Started

Most users only need to clone this repo and build. The FFI bindings are pre-generated and committed, and the vendor `.so` libraries are already installed on the reCamera device.

### For app developers

Add this SDK as a dependency in your own project. No SDK download required -- the FFI bindings are pre-generated and the vendor `.so` libraries are already on the reCamera device.

```toml
[dependencies]
recamera = { git = "https://github.com/anthropics/recamera-rs", features = ["camera", "uart"] }
```

When cross-compiling your application for the reCamera target (`riscv64gc-unknown-linux-musl`), the linker needs the vendor `.so` files at build time. Download the reCamera-OS SDK from [reCamera-OS releases](https://github.com/Seeed-Studio/reCamera-OS/releases) (look for `*_sdk.tar.gz`) and set `SG200X_SDK_PATH` to the extracted path. The `build.rs` script finds the libraries at `$SG200X_SDK_PATH/cvi_mpi/lib/` automatically.

### For SDK maintainers

See [docs/MAINTAINER.md](docs/MAINTAINER.md) for instructions on regenerating FFI bindings and other development tasks.

## Supported Platforms

This SDK is designed to be built on **macOS** or **Linux** and cross-compiled for the reCamera (RISC-V 64-bit, musl libc). All paths and scripts are portable -- there are no host-specific configurations in the codebase.

## License

Licensed under either of:

- MIT license
- Apache License, Version 2.0

at your option.

## Contributing

Contributions, issues, and suggestions are welcome. Areas where help is especially useful include camera and inference integration, cross-compilation tooling, and documentation.
