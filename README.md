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

## Generating FFI Bindings

The `camera` and `infer` features require FFI bindings for the SG2002 vendor C libraries. Pre-generated bindings are committed to the repo, but if you need to regenerate them:

1. Download the SDK tarball from [reCamera-OS releases](https://github.com/Seeed-Studio/reCamera-OS/releases) (look for `*_sdk.tar.gz`) and extract it.

2. Install bindgen:
   ```sh
   cargo install bindgen-cli
   ```

3. Run the generation script:
   ```sh
   SDK_PATH=/path/to/sg2002_recamera_emmc ./scripts/generate-bindings.sh
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
