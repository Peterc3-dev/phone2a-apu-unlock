#!/usr/bin/env bash
# first-contact.sh — push apu650-probe to the phone and run the handshake.
#
# Prereqs on the phone:
#   1. Bootloader unlocked
#   2. KernelSU (or Magisk) flashed and working — `adb shell su -c id` returns uid=0
#   3. apu650-unlock-ksu-v0.3.zip flashed in KernelSU manager → reboot
#
# Prereqs on the host:
#   - aarch64-linux-android binary at apu650-probe/target/aarch64-linux-android/release/apu650-probe
#     Build with:  cd apu650-probe && cargo build --release --target aarch64-linux-android
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${BIN:-$REPO_ROOT/apu650-probe/target/aarch64-linux-android/release/apu650-probe}"
DEST="/data/local/tmp/apu650-probe"

log() { printf '\e[1;32m[first-contact]\e[0m %s\n' "$*"; }
err() { printf '\e[1;31m[error]\e[0m %s\n' "$*" >&2; exit 1; }

[ -x "$BIN" ] || err "binary not built: $BIN  (cd apu650-probe && cargo build --release --target aarch64-linux-android)"

log "checking adb"
adb get-state >/dev/null 2>&1 || err "no device — is the phone plugged in with USB debugging on?"
DEV="$(adb get-serialno)"
log "device: $DEV"

log "checking root"
ID=$(adb shell su -c id 2>&1 || true)
case "$ID" in
    *uid=0*) log "su works: $ID" ;;
    *) err "root not available — install KernelSU or Magisk first. got: $ID" ;;
esac

log "checking SELinux state"
ENF=$(adb shell getenforce 2>&1)
log "getenforce: $ENF"

log "checking /dev/apusys"
NODE_LS=$(adb shell su -c 'ls -lZ /dev/apusys 2>&1' || true)
log "node: $NODE_LS"
case "$NODE_LS" in
    *No\ such\ file*) err "/dev/apusys missing — kernel module not loaded" ;;
    *apusys_device*)  log "node label looks correct" ;;
esac

log "checking sepolicy patch (apu650-unlock module flashed?)"
SEPOL_CHECK=$(adb shell su -c 'sesearch -A -s shell -t apusys_device -p open 2>&1' || true)
case "$SEPOL_CHECK" in
    *allow*shell*apusys_device*) log "sepolicy allows shell→apusys: OK" ;;
    *not\ found*|*"sesearch: not found"*) log "sesearch not on device, skipping policy verify" ;;
    *) log "couldn't verify sepolicy via sesearch — will catch any EACCES at run time" ;;
esac

log "pushing binary"
adb push "$BIN" "$DEST" >/dev/null
adb shell chmod 0755 "$DEST"

log "running probe (output below the line)"
echo "--------------------------------------------------------------------"
adb shell su -c "$DEST"
RC=$?
echo "--------------------------------------------------------------------"
if [ $RC -eq 0 ]; then
    log "handshake completed cleanly — version + bitmasks above"
else
    err "probe exited $RC — check dmesg with: adb shell su -c 'dmesg | tail -50'"
fi

log "next: tap real ioctl traffic with frida (see scripts/frida-tap.sh)"
