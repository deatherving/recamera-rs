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

### For app developers (pure-Rust features only)

No SDK required. Works on macOS and Linux:

```sh
cargo build
cargo test
```

### For app developers (cross-compiling for reCamera)

To cross-compile binaries that use `camera` or `infer`, the Rust linker needs the vendor `.so` libraries at build time. These come from the reCamera-OS SDK. The reCamera device itself already has these libraries installed -- the SDK is only needed on your build machine.

1. Download the reCamera-OS SDK from [reCamera-OS releases](https://github.com/Seeed-Studio/reCamera-OS/releases) (look for `*_sdk.tar.gz`) and extract it anywhere.

2. Install the RISC-V target:
   ```sh
   rustup target add riscv64gc-unknown-linux-musl
   ```

3. Build with the SDK path:
   ```sh
   SG200X_SDK_PATH=/path/to/sg2002_recamera_emmc \
     cargo build --target riscv64gc-unknown-linux-musl --release
   ```

The `build.rs` script finds the vendor libraries at `$SG200X_SDK_PATH/cvi_mpi/lib/` and links them automatically.

Pure-Rust features can be cross-compiled without the SDK:

```sh
cargo build --target riscv64gc-unknown-linux-musl --release \
  -p recamera --no-default-features --features "uart,storage,logging,config,system"
```

### For SDK maintainers (regenerating FFI bindings)

The pre-generated bindings are committed to the repo. You only need to regenerate them when the reCamera-OS SDK is updated with new headers.

The script `scripts/generate-bindings.sh` is included for this purpose. It uses `bindgen` to produce `crates/recamera-cvi-sys/src/bindings.rs` from the SDK headers.

1. Download the reCamera-OS SDK (same as above).

2. Install bindgen:
   ```sh
   cargo install bindgen-cli
   ```

3. Run the generation script:
   ```sh
   ./scripts/generate-bindings.sh /path/to/sg2002_recamera_emmc
   ```

4. Verify and commit:
   ```sh
   cargo check -p recamera-cvi-sys
   git add crates/recamera-cvi-sys/src/bindings.rs
   git commit -m "feat: update FFI bindings"
   ```

## Supported Platforms

This SDK is designed to be built on **macOS** or **Linux** and cross-compiled for the reCamera (RISC-V 64-bit, musl libc). All paths and scripts are portable -- there are no host-specific configurations in the codebase.

## License

Licensed under either of:

- MIT license
- Apache License, Version 2.0

at your option.

## Contributing

Contributions, issues, and suggestions are welcome. Areas where help is especially useful include camera and inference integration, cross-compilation tooling, and documentation.
