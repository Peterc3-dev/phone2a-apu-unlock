#!/usr/bin/env bash
# compile_dla.sh — wrap ncc-tflite (Genio SDK) with sane defaults and
# automatic LD_LIBRARY_PATH so the cross-compiler binary runs on a
# desktop Linux host without a Yocto rootfs.
#
# Usage:
#     ./compile_dla.sh <model.tflite> [arch=mdla3.0]
#
# Env overrides:
#     NCC_TFLITE   — path to ncc-tflite binary
#     GENIO_LIB    — directory with Genio SDK shared libs (libneuron*.so etc.)
set -euo pipefail

MODEL="${1:?usage: $0 <model.tflite> [arch=mdla3.0]}"
ARCH="${2:-mdla3.0}"
OUT="${MODEL%.tflite}.dla"

# Best-effort discovery — adjust if you extracted the SDK to a different path.
NCC_TFLITE="${NCC_TFLITE:-}"
GENIO_LIB="${GENIO_LIB:-}"

if [ -z "$NCC_TFLITE" ]; then
    for cand in \
        "$HOME/genio-sdk/usr/bin/ncc-tflite" \
        "/opt/genio-sdk/usr/bin/ncc-tflite" \
        "$(command -v ncc-tflite 2>/dev/null || true)"; do
        if [ -x "$cand" ]; then NCC_TFLITE="$cand"; break; fi
    done
fi
[ -x "${NCC_TFLITE:-}" ] || {
    cat >&2 <<EOF
ncc-tflite not found.
Either:
  - download the Genio 700 EVK Yocto image from
    https://download01.mediatek.com/aiot/  (registration required)
  - extract /usr/bin/ncc-tflite + /usr/lib/libneuron*.so
  - set NCC_TFLITE and GENIO_LIB to those paths
See dla/FETCH_SDK.md for the full procedure.
EOF
    exit 2
}

if [ -z "$GENIO_LIB" ]; then
    GENIO_LIB="$(dirname "$NCC_TFLITE")/../lib"
fi

[ -d "$GENIO_LIB" ] || { echo "GENIO_LIB not a dir: $GENIO_LIB" >&2; exit 2; }

echo "[compile_dla] ncc-tflite: $NCC_TFLITE"
echo "[compile_dla] libs:       $GENIO_LIB"
echo "[compile_dla] arch:       $ARCH"
echo "[compile_dla] in:         $MODEL"
echo "[compile_dla] out:        $OUT"

LD_LIBRARY_PATH="$GENIO_LIB:${LD_LIBRARY_PATH:-}" \
    "$NCC_TFLITE" --arch="$ARCH" -o "$OUT" "$MODEL"

ls -lh "$OUT"
echo "[compile_dla] first bytes:"
xxd "$OUT" | head -4
