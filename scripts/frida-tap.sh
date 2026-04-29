#!/usr/bin/env bash
# frida-tap.sh — capture real APU ioctl traffic from the NN HAL service while
# you provoke it (e.g. take an Ultra HDR shot).
#
# Prereqs on the phone:
#   - frida-server-aarch64 pushed to /data/local/tmp/ and running as root
#     (https://github.com/frida/frida/releases — pick the matching version)
#
# Prereqs on the host:
#   - frida CLI:  pipx install frida-tools  (or pip install --user frida-tools)
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
JS="$REPO_ROOT/frida/apu_ioctl_tap.js"
[ -f "$JS" ] || { echo "missing $JS"; exit 1; }
command -v frida >/dev/null || { echo "frida CLI missing — pipx install frida-tools"; exit 1; }

# Discover the NN HAL service PID on the phone (name varies by Android version).
PID=$(adb shell su -c 'pidof android.hardware.neuralnetworks-shim-service-mtk 2>/dev/null || \
                       pidof android.hardware.neuralnetworks@1.3-service.mediatek 2>/dev/null || \
                       pidof neuralnetworks_hal_service 2>/dev/null' | tr -d '\r')

if [ -z "$PID" ]; then
    echo "couldn't find NN HAL pid — list candidates:"
    adb shell su -c 'ps -A | grep -iE "neural|apusys|neuropilot"'
    exit 1
fi
echo "[frida-tap] attaching to PID $PID (NN HAL)"
echo "[frida-tap] now provoke the APU on the phone — open Camera, take an Ultra HDR shot"
echo "[frida-tap] trace appended to /sdcard/apu_ioctl.jsonl on the device"
echo "[frida-tap] Ctrl-C when done; pull with: adb pull /sdcard/apu_ioctl.jsonl"
echo

frida -U -p "$PID" -l "$JS" --no-pause
