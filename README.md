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

## Generating FFI Bindings

Pre-generated bindings are committed to the repo, so most users don't need to do this. If you need to regenerate them (e.g., for a new SDK version):

1. Download the reCamera-OS SDK from [reCamera-OS releases](https://github.com/Seeed-Studio/reCamera-OS/releases) (look for `*_sdk.tar.gz`):
   ```sh
   mkdir -p sdk && cd sdk
   # Download sg2002_reCamera_*_sdk.tar.gz from the releases page
   tar xzf sg2002_reCamera_*_sdk.tar.gz
   cd ..
   ```

2. Install bindgen:
   ```sh
   cargo install bindgen-cli
   ```

3. Run the generation script:
   ```sh
   SDK_PATH=./sdk/sg2002_recamera_emmc ./scripts/generate-bindings.sh
   ```

4. Verify and commit:
   ```sh
   cargo check -p recamera-cvi-sys
   git add crates/recamera-cvi-sys/src/bindings.rs
   git commit -m "feat: update FFI bindings"
   ```

Pure-Rust crates (uart, storage, logging, config, system) do not require the SDK.

## Cross-Compilation

reCamera uses the SG2002 SoC (RISC-V 64-bit). To cross-compile:

```sh
# Install the target
rustup target add riscv64gc-unknown-linux-musl

# Build (set SG200X_SDK_PATH to the SDK sysroot for camera/infer linking)
export SG200X_SDK_PATH=/path/to/sg2002_recamera_emmc
cargo build --target riscv64gc-unknown-linux-musl --release
```

Pure-Rust crates (uart, storage, logging, config, system) can be cross-compiled without the SDK.

## License

Licensed under either of:

- MIT license
- Apache License, Version 2.0

at your option.

## Contributing

Contributions, issues, and suggestions are welcome. Areas where help is especially useful include camera and inference integration, cross-compilation tooling, and documentation.
