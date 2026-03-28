#!/usr/bin/env bash
#
# Generate FFI bindings for the SG2002 CVI libraries.
#
# Prerequisites:
#   1. Install bindgen-cli:  cargo install bindgen-cli
#   2. Download the reCamera-OS SDK from:
#      https://github.com/Seeed-Studio/reCamera-OS/releases
#      Look for *_sdk.tar.gz and extract it.
#   3. Set SDK_PATH to the root of the extracted SDK
#
# Usage:
#   SDK_PATH=/path/to/sg2002_recamera_emmc ./scripts/generate-bindings.sh
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
    echo "Download the reCamera-OS SDK from:"
    echo "  https://github.com/Seeed-Studio/reCamera-OS/releases"
    echo ""
    echo "Extract it and run:"
    echo "  SDK_PATH=/path/to/sg2002_recamera_emmc $0"
    exit 1
fi

# --- Detect SDK layout ---

MPI_INCLUDE=""
NN_INCLUDE=""

# reCamera-OS SDK tarball:
#   <SDK_PATH>/cvi_mpi/include/
#   <SDK_PATH>/tpu_sdk/include/  (or cviruntime/include/)
if [ -d "$SDK_PATH/cvi_mpi/include" ]; then
    MPI_INCLUDE="$SDK_PATH/cvi_mpi/include"
    if [ -d "$SDK_PATH/tpu_sdk/include" ]; then
        NN_INCLUDE="$SDK_PATH/tpu_sdk/include"
    elif [ -d "$SDK_PATH/cviruntime/include" ]; then
        NN_INCLUDE="$SDK_PATH/cviruntime/include"
    fi
fi

if [ -z "$MPI_INCLUDE" ]; then
    echo "Error: Could not find CVI MPI headers in SDK_PATH=$SDK_PATH"
    echo ""
    echo "Expected: $SDK_PATH/cvi_mpi/include/"
    echo ""
    echo "Make sure you downloaded and extracted the reCamera-OS SDK tarball."
    exit 1
fi

if [ -z "$NN_INCLUDE" ]; then
    echo "Warning: CVI runtime (NPU) headers not found. Inference bindings will be skipped."
    echo "Looked for:"
    echo "  $SDK_PATH/tpu_sdk/include/"
    echo "  $SDK_PATH/cviruntime/include/"
    echo ""
fi

echo "SDK detected:"
echo "  MPI headers:     $MPI_INCLUDE"
echo "  Runtime headers: ${NN_INCLUDE:-<not found>}"

if ! command -v bindgen &>/dev/null; then
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
 * Includes the CVI MPI and CVI runtime headers needed for
 * camera capture and NPU inference on the SG2002 SoC.
 */

/* Base types and system */
#include "cvi_type.h"
#include "cvi_sys.h"
#include "cvi_vb.h"

/* Video input */
#include "cvi_vi.h"

/* Video processing */
#include "cvi_vpss.h"

/* Video encoding */
#include "cvi_venc.h"
HEADER

    if [ -n "$NN_INCLUDE" ]; then
        echo ""
        echo "/* NPU inference runtime */"
        echo "#include \"cviruntime.h\""
    fi
} > "$WRAPPER"

# --- Build include flags ---

INCLUDE_FLAGS="-I$MPI_INCLUDE"
if [ -n "$NN_INCLUDE" ]; then
    INCLUDE_FLAGS="$INCLUDE_FLAGS -I$NN_INCLUDE"
fi

# --- Run bindgen ---

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
