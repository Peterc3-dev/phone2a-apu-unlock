#!/usr/bin/env bash
# extract_genio_sdk.sh — pull only the bits we need from a Genio Yocto image.
#
# Inputs accepted (auto-detected):
#   - genio-700-evk_image-yocto-v24.0.tar.gz   (full rootfs tarball)
#   - rootfs.ext4 / rootfs.squashfs / rootfs.cpio.gz
#   - already-extracted directory containing usr/bin/ncc-tflite
#
# Output: $OUT (default: ~/projects/apu650-probe/external/genio-sdk/)
#   ├── bin/ncc-tflite          (the TFLite→DLA compiler)
#   ├── bin/neuronrt            (MTK reference runtime, useful for first smoke)
#   └── lib/libneuron*.so + libapunoltool.so + ...
#
# Usage:
#     ./extract_genio_sdk.sh <archive_or_dir>
#     ./extract_genio_sdk.sh ~/Downloads/genio-700-evk_yocto.tar.gz
set -euo pipefail

SRC="${1:?usage: $0 <genio-rootfs-archive-or-dir>}"
OUT="${OUT:-$REPO_ROOT/external/genio-sdk}"

log() { printf '\e[1;32m[extract]\e[0m %s\n' "$*"; }
err() { printf '\e[1;31m[error]\e[0m %s\n' "$*" >&2; exit 1; }

mkdir -p "$OUT/bin" "$OUT/lib"

# Stage A: get the rootfs tree into a workdir we can walk.
WORK="$(mktemp -d -t genio-extract-XXXXXX)"
trap 'rm -rf "$WORK"' EXIT INT TERM

if [ -d "$SRC" ]; then
    log "source is a directory; using directly"
    ROOTFS="$SRC"
elif [[ "$SRC" == *.tar.gz || "$SRC" == *.tgz ]]; then
    log "extracting tarball into workdir"
    tar -C "$WORK" -xzf "$SRC"
    # Some tarballs nest ./rootfs, others extract flat. Find the bin/ncc-tflite.
    ROOTFS="$(find "$WORK" -maxdepth 5 -type f -name ncc-tflite -printf '%h\n' | head -1)"
    [ -z "$ROOTFS" ] || ROOTFS="$(dirname "$ROOTFS")/.."
elif [[ "$SRC" == *.tar.bz2 || "$SRC" == *.tbz2 ]]; then
    tar -C "$WORK" -xjf "$SRC"
    ROOTFS="$(find "$WORK" -maxdepth 5 -type f -name ncc-tflite -printf '%h\n' | head -1)"
    [ -z "$ROOTFS" ] || ROOTFS="$(dirname "$ROOTFS")/.."
elif [[ "$SRC" == *.ext4 || "$SRC" == *.img ]]; then
    log "mounting rootfs image"
    sudo mkdir -p /mnt/genio-rootfs
    sudo mount -o loop,ro "$SRC" /mnt/genio-rootfs
    trap 'sudo umount /mnt/genio-rootfs 2>/dev/null; rm -rf "$WORK"' EXIT INT TERM
    ROOTFS=/mnt/genio-rootfs
elif [[ "$SRC" == *.squashfs ]]; then
    log "extracting squashfs"
    command -v unsquashfs >/dev/null || err "install squashfs-tools (pacman -S squashfs-tools)"
    unsquashfs -d "$WORK/rootfs" "$SRC" >/dev/null
    ROOTFS="$WORK/rootfs"
else
    err "unrecognised input: $SRC"
fi

[ -n "${ROOTFS:-}" ] && [ -d "$ROOTFS" ] || err "could not locate the rootfs containing usr/bin/ncc-tflite"
log "rootfs at: $ROOTFS"

# Stage B: locate ncc-tflite.
NCC="$(find "$ROOTFS" -type f -name ncc-tflite | head -1)"
[ -n "$NCC" ] || err "ncc-tflite not found anywhere under $ROOTFS"
log "found ncc-tflite at $NCC"
RTFS="$(dirname "$NCC")/.."

# Copy the binaries we want.
for b in ncc-tflite neuronrt apunoltool; do
    if [ -f "$RTFS/bin/$b" ]; then
        cp -v "$RTFS/bin/$b" "$OUT/bin/" 2>&1 | head -1
    elif [ -f "$RTFS/usr/bin/$b" ]; then
        cp -v "$RTFS/usr/bin/$b" "$OUT/bin/" 2>&1 | head -1
    fi
done

# Resolve the shared libs ncc-tflite + neuronrt actually need, including transitives.
log "resolving shared libs"
declare -A SEEN=()
queue=("$OUT/bin/ncc-tflite" "$OUT/bin/neuronrt")
for entry in "${queue[@]}"; do
    [ -f "$entry" ] || continue
    needed="$(readelf -d "$entry" 2>/dev/null | awk '/NEEDED/{gsub(/[\[\]]/,"",$5); print $5}')"
    for n in $needed; do
        [ -n "${SEEN[$n]:-}" ] && continue
        SEEN[$n]=1
        # Find the lib in the rootfs lib/ and usr/lib/
        for prefix in "$RTFS/lib" "$RTFS/usr/lib" "$RTFS/lib/aarch64-linux-gnu" "$RTFS/usr/lib/aarch64-linux-gnu"; do
            if [ -f "$prefix/$n" ]; then
                cp -L "$prefix/$n" "$OUT/lib/"
                # Recurse into transitive deps.
                queue+=("$OUT/lib/$n")
                break
            fi
        done
    done
done

log "extracted artifacts:"
ls -lh "$OUT/bin/" "$OUT/lib/" | head -40

cat > "$OUT/env.sh" <<EOF
# source this to use the extracted SDK
export GENIO_SDK="$OUT"
export PATH="\$GENIO_SDK/bin:\$PATH"
export LD_LIBRARY_PATH="\$GENIO_SDK/lib:\${LD_LIBRARY_PATH:-}"
EOF

log "done. To use:"
log "  source $OUT/env.sh"
log "  ncc-tflite --help | head"
