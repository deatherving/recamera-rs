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
- FFI crates (`camera`, `infer`, `cvi-sys`) are scaffolded but pending real bindings against the CVI runtime libraries.

## Cross-Compilation

reCamera uses the SG2002 SoC (RISC-V 64-bit). To cross-compile:

```sh
# Install the target
rustup target add riscv64gc-unknown-linux-musl

# Build (set SG200X_SDK_PATH to your toolchain sysroot)
export SG200X_SDK_PATH=/path/to/sg200x-sdk
cargo build --target riscv64gc-unknown-linux-musl
```

## License

Licensed under either of:

- MIT license
- Apache License, Version 2.0

at your option.

## Contributing

Contributions, issues, and suggestions are welcome. Areas where help is especially useful include camera and inference integration, cross-compilation tooling, and documentation.
