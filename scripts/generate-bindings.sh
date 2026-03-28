#!/usr/bin/env bash
#
# Generate FFI bindings for the SG2002 CVI libraries.
#
# Prerequisites:
#   1. Install bindgen-cli:  cargo install bindgen-cli
#   2. Obtain the SDK headers from one of:
#      - Milk-V Duo SDK: git clone --depth 1 https://github.com/milkv-duo/duo-buildroot-sdk
#      - reCamera-OS build output: output/staging/usr/include/
#      - sscma-example-sg200x SDK download
#   3. Set SDK_PATH to the root of the SDK (containing middleware/ and cviruntime/ dirs)
#
# Usage:
#   SDK_PATH=/path/to/duo-buildroot-sdk ./scripts/generate-bindings.sh
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
    echo "Set it to the root of the Milk-V Duo SDK or equivalent:"
    echo "  SDK_PATH=/path/to/duo-buildroot-sdk $0"
    echo ""
    echo "The SDK should contain:"
    echo "  middleware/v2/include/   (CVI MPI headers)"
    echo "  cviruntime/include/     (NPU inference headers)"
    exit 1
fi

MPI_INCLUDE="$SDK_PATH/middleware/v2/include"
NN_INCLUDE="$SDK_PATH/cviruntime/include"

if [ ! -d "$MPI_INCLUDE" ]; then
    echo "Error: MPI headers not found at $MPI_INCLUDE"
    echo "Expected directory: \$SDK_PATH/middleware/v2/include/"
    exit 1
fi

if [ ! -d "$NN_INCLUDE" ]; then
    echo "Error: CVI runtime headers not found at $NN_INCLUDE"
    echo "Expected directory: \$SDK_PATH/cviruntime/include/"
    exit 1
fi

if ! command -v bindgen &>/dev/null; then
    echo "Error: bindgen-cli is not installed."
    echo "Install it with: cargo install bindgen-cli"
    exit 1
fi

# --- Create wrapper header ---

cat > "$WRAPPER" << 'EOF'
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

/* NPU inference runtime */
#include "cviruntime.h"
EOF

echo "Generating bindings..."

bindgen "$WRAPPER" \
    -o "$OUTPUT" \
    --use-core \
    --no-layout-tests \
    --default-enum-style rust \
    --bitfield-enum ".*" \
    --allowlist-function "CVI_SYS_.*" \
    --allowlist-function "CVI_VB_.*" \
    --allowlist-function "CVI_VI_.*" \
    --allowlist-function "CVI_VPSS_.*" \
    --allowlist-function "CVI_VENC_.*" \
    --allowlist-function "CVI_NN_.*" \
    --allowlist-type "CVI_.*" \
    --allowlist-type "VIDEO_FRAME.*" \
    --allowlist-type "PIXEL_FORMAT.*" \
    --allowlist-type "SIZE_S" \
    --allowlist-type "FRAME_RATE_CTRL.*" \
    --allowlist-type "MMF_CHN.*" \
    --allowlist-type "MOD_ID.*" \
    --allowlist-type "VB_.*" \
    --allowlist-type "VI_.*" \
    --allowlist-type "VPSS_.*" \
    --allowlist-type "VENC_.*" \
    --allowlist-type "PAYLOAD_TYPE.*" \
    --allowlist-var "CVI_.*" \
    -- \
    -I"$MPI_INCLUDE" \
    -I"$NN_INCLUDE" \
    --target=riscv64-unknown-linux-musl

# Clean up wrapper header (it's generated, not committed)
rm -f "$WRAPPER"

echo "Bindings written to: $OUTPUT"
echo ""
echo "Next steps:"
echo "  1. Review the generated bindings"
echo "  2. Run: cargo check -p recamera-cvi-sys"
echo "  3. Commit the updated bindings.rs"
