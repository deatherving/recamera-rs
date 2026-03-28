# SDK Maintainer Guide

This document covers development tasks for maintaining the recamera-rs SDK.

## Regenerating FFI Bindings

The pre-generated bindings are committed to the repo at `crates/recamera-cvi-sys/src/bindings.rs`. You only need to regenerate them when the reCamera-OS SDK is updated with new headers.

### Prerequisites

1. Download the reCamera-OS SDK from [reCamera-OS releases](https://github.com/Seeed-Studio/reCamera-OS/releases) (look for `*_sdk.tar.gz`) and extract it anywhere.

2. Install bindgen:
   ```sh
   cargo install bindgen-cli
   ```

### Generate

Run the script, passing the path to your extracted SDK:

```sh
./scripts/generate-bindings.sh /path/to/sg2002_recamera_emmc
```

The script will:
- Auto-detect the SDK layout (headers at `cvi_mpi/include/`, sysroot at `buildroot-2021.05/...`)
- Generate bindings for all CVI MPI functions (SYS, VB, VI, VPSS, VENC)
- Include NPU inference bindings if the runtime headers are available in the SDK

### Verify and commit

```sh
cargo check -p recamera-cvi-sys
cargo test --workspace
git add crates/recamera-cvi-sys/src/bindings.rs
git commit -m "feat: update FFI bindings"
```

## SDK Directory Structure

The reCamera-OS SDK tarball contains:

```
sg2002_recamera_emmc/
  cvi_mpi/
    include/         # C headers (used by bindgen)
    lib/             # Pre-built .so files (used by linker at cross-compile time)
  buildroot-2021.05/ # Sysroot with linux kernel headers
```

## Running Tests

```sh
cargo test --workspace
cargo clippy --workspace --all-features -- -D warnings
cargo fmt --all -- --check
```
