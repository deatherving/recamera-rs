# recamera-rs

A Rust SDK for [Seeed reCamera](https://wiki.seeedstudio.com/recamera/) -- camera capture, local inference, serial I/O, storage, and system utilities for edge vision applications on the SG2002 SoC.

> This is a community project and is not affiliated with or officially maintained by Seeed Studio.

## Usage

Add `recamera` as a dependency in your project:

```toml
[dependencies]
recamera = { git = "https://github.com/deatherving/recamera-rs", features = ["camera", "uart"] }
```

No SDK download required. The vendor libraries are loaded at runtime on the reCamera device.

```rust
use recamera::camera::{Camera, CameraConfig};

let mut camera = Camera::new(CameraConfig::default())?;
camera.start_stream()?;
let frame = camera.capture()?;
```

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
| `full`    | Enables all features                     | No      |

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
| `recamera-system`  | Device info, LED control, system utilities              |

## How It Works

The vendor C libraries (camera, video, NPU inference) are loaded at **runtime** on the reCamera device using `dlopen`. No compile-time linking or SDK download is needed to build your application.

`recamera-cvi-sys` provides:
- Type definitions, structs, enums, and constants generated from the SDK headers
- A runtime loader (`CviLibs`) that finds and loads the vendor `.so` libraries on the device

The higher-level crates (`recamera-camera`, `recamera-infer`) wrap the loader with safe Rust APIs.

## License

Licensed under either of:

- [MIT license](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.

## Contributing

Contributions, issues, and suggestions are welcome.
