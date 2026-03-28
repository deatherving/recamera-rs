#!/usr/bin/env bash
#
# Generate FFI bindings for the SG2002 CVI libraries.
#
# Prerequisites:
#   1. Install Rust and bindgen-cli:
#      cargo install bindgen-cli
#   2. Download the reCamera-OS SDK from:
#      https://github.com/Seeed-Studio/reCamera-OS/releases
#      Look for *_sdk.tar.gz and extract it into the sdk/ directory:
#        mkdir -p sdk && cd sdk
#        tar xzf sg2002_reCamera_*_sdk.tar.gz
#   3. Run this script:
#      SDK_PATH=./sdk/sg2002_recamera_emmc ./scripts/generate-bindings.sh
#
# The generated bindings are written to crates/recamera-cvi-sys/src/bindings.rs
# and should be committed to the repository.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
OUTPUT="$PROJECT_ROOT/crates/recamera-cvi-sys/src/bindings.rs"
WRAPPER="$PROJECT_ROOT/crates/recamera-cvi-sys/wrapper.h"

# --- Validate environment ---

if [ -z "${SDK_PATH:-}" ]; then
    echo "Error: SDK_PATH is not set."
    echo ""
    echo "1. Download the reCamera-OS SDK from:"
    echo "   https://github.com/Seeed-Studio/reCamera-OS/releases"
    echo "   (look for *_sdk.tar.gz)"
    echo ""
    echo "2. Extract it:"
    echo "   mkdir -p sdk && cd sdk"
    echo "   tar xzf sg2002_reCamera_*_sdk.tar.gz"
    echo ""
    echo "3. Run this script:"
    echo "   SDK_PATH=./sdk/sg2002_recamera_emmc $0"
    exit 1
fi

# --- Detect SDK headers ---

MPI_INCLUDE="$SDK_PATH/cvi_mpi/include"
MPI_LINUX_INCLUDE="$SDK_PATH/cvi_mpi/include/linux"

if [ ! -d "$MPI_INCLUDE" ]; then
    echo "Error: CVI MPI headers not found at $MPI_INCLUDE"
    echo ""
    echo "Make sure SDK_PATH points to the extracted SDK root, e.g.:"
    echo "  SDK_PATH=./sdk/sg2002_recamera_emmc"
    exit 1
fi

if [ ! -d "$MPI_LINUX_INCLUDE" ]; then
    echo "Error: CVI common headers not found at $MPI_LINUX_INCLUDE"
    exit 1
fi

# NPU inference headers (optional — not included in current SDK release)
NN_INCLUDE=""
if [ -d "$SDK_PATH/tpu_sdk/include" ]; then
    NN_INCLUDE="$SDK_PATH/tpu_sdk/include"
elif [ -d "$SDK_PATH/cviruntime/include" ]; then
    NN_INCLUDE="$SDK_PATH/cviruntime/include"
fi

echo "SDK detected:"
echo "  MPI headers:     $MPI_INCLUDE"
echo "  Common headers:  $MPI_LINUX_INCLUDE"
if [ -n "$NN_INCLUDE" ]; then
    echo "  NPU headers:     $NN_INCLUDE"
else
    echo "  NPU headers:     not found (inference bindings will be skipped)"
fi

if ! command -v bindgen &>/dev/null; then
    echo ""
    echo "Error: bindgen-cli is not installed."
    echo "Install it with: cargo install bindgen-cli"
    exit 1
fi

# --- Create wrapper header ---

{
    cat << 'HEADER'
/**
 * Wrapper header for bindgen.
 *
 * Includes the CVI MPI headers needed for camera capture
 * and video processing on the SG2002 SoC.
 */

/* Common types (cvi_type.h, cvi_defines.h, etc.) */
#include "cvi_type.h"
#include "cvi_common.h"
#include "cvi_comm_sys.h"
#include "cvi_comm_vb.h"
#include "cvi_comm_video.h"
#include "cvi_comm_vi.h"
#include "cvi_comm_vpss.h"
#include "cvi_comm_venc.h"

/* API functions */
#include "cvi_sys.h"
#include "cvi_vb.h"
#include "cvi_vi.h"
#include "cvi_vpss.h"
#include "cvi_venc.h"
HEADER

    if [ -n "$NN_INCLUDE" ]; then
        echo ""
        echo "/* NPU inference runtime */"
        echo "#include \"cviruntime.h\""
    fi
} > "$WRAPPER"

# --- Detect sysroot (for linux kernel headers) ---

SYSROOT=""
SYSROOT_CANDIDATE="$SDK_PATH/buildroot-2021.05/output/cvitek_CV181X_musl_riscv64/host/riscv64-buildroot-linux-musl/sysroot"
if [ -d "$SYSROOT_CANDIDATE/usr/include" ]; then
    SYSROOT="$SYSROOT_CANDIDATE"
    echo "  Sysroot:         $SYSROOT"
fi

# --- Build include flags ---

INCLUDE_FLAGS="-I$MPI_INCLUDE -I$MPI_LINUX_INCLUDE"
if [ -n "$NN_INCLUDE" ]; then
    INCLUDE_FLAGS="$INCLUDE_FLAGS -I$NN_INCLUDE"
fi
if [ -n "$SYSROOT" ]; then
    INCLUDE_FLAGS="$INCLUDE_FLAGS --sysroot=$SYSROOT -isystem $SYSROOT/usr/include"
fi

# --- Run bindgen ---

echo ""
echo "Generating bindings..."

ALLOWLIST_ARGS=(
    --allowlist-function "CVI_SYS_.*"
    --allowlist-function "CVI_VB_.*"
    --allowlist-function "CVI_VI_.*"
    --allowlist-function "CVI_VPSS_.*"
    --allowlist-function "CVI_VENC_.*"
    --allowlist-type "CVI_.*"
    --allowlist-type "VIDEO_FRAME.*"
    --allowlist-type "PIXEL_FORMAT.*"
    --allowlist-type "SIZE_S"
    --allowlist-type "FRAME_RATE_CTRL.*"
    --allowlist-type "MMF_CHN.*"
    --allowlist-type "MOD_ID.*"
    --allowlist-type "VB_.*"
    --allowlist-type "VI_.*"
    --allowlist-type "VPSS_.*"
    --allowlist-type "VENC_.*"
    --allowlist-type "PAYLOAD_TYPE.*"
    --allowlist-var "CVI_.*"
)

if [ -n "$NN_INCLUDE" ]; then
    ALLOWLIST_ARGS+=(
        --allowlist-function "CVI_NN_.*"
    )
fi

bindgen "$WRAPPER" \
    -o "$OUTPUT" \
    --use-core \
    --no-layout-tests \
    --default-enum-style rust \
    "${ALLOWLIST_ARGS[@]}" \
    -- \
    $INCLUDE_FLAGS \
    --target=riscv64-unknown-linux-musl

# Clean up wrapper header
rm -f "$WRAPPER"

echo ""
echo "Bindings written to: $OUTPUT"
echo ""
echo "Next steps:"
echo "  1. Review the generated bindings"
echo "  2. Run: cargo check -p recamera-cvi-sys"
echo "  3. Commit the updated bindings.rs"
