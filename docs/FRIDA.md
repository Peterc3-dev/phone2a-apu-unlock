# Frida ioctl tap — when, why, how

`frida/apu_ioctl_tap.js` hooks `libc.ioctl()` in any process and logs every ioctl whose target fd points to a `/dev/(apu|mdla|vpu|mtk_aov|edma|mdw)*` node. `frida/decode.py` joins the captured trace against `abi/apusys_ioctl_abi.json` and prints a labelled, ordered trace.

This is not part of the unlock flow. It's a debugging tool you reach for when:

1. The handshake works but `MEM(Alloc)` or `CMD(Run)` returns `-EFAULT` — meaning some struct in `apu650-probe/src/uapi.rs` is the wrong size for your device's midware revision.
2. You're trying to learn what the vendor stack actually does — what cmdbuf payloads NeuroPilot sends when you take an Ultra HDR photo, etc.
3. You're verifying that a different regional build of Phone 2a uses the same cmd numbers and struct layouts.

## How it works

The script attaches `Interceptor.attach()` to `libc.so:ioctl`. On every call:

1. Resolve the fd to a path via `readlink("/proc/self/fd/<n>")`.
2. Filter — only path prefixes `/dev/(apu|mdla|vpu|mtk_aov|edma|mdw)` are interesting.
3. Decode the cmd number into `(dir, size, type, nr)` — the `_IOC_*` macro components.
4. Hexdump the first `min(size, 256)` bytes pointed to by `args[2]` (the union arg).
5. Append one JSON record to `/sdcard/apu_ioctl.jsonl`.

The phone-side process keeps writing as long as Frida is attached. Detach (Ctrl-C the host CLI) to stop capture.

## Setup

Host:
```sh
pipx install frida-tools
```

Phone (rooted):
```sh
# Pick the frida-server-aarch64 release matching your host's frida-tools version.
adb push frida-server-aarch64 /data/local/tmp/
adb shell su -c 'chmod 0755 /data/local/tmp/frida-server-aarch64'
adb shell su -c 'nohup /data/local/tmp/frida-server-aarch64 >/dev/null 2>&1 &'
```

## Run

```sh
scripts/frida-tap.sh
# (provoke the APU on the phone — open Camera, take an Ultra HDR shot)
# Ctrl-C when done

adb pull /sdcard/apu_ioctl.jsonl
frida/decode.py apu_ioctl.jsonl abi/apusys_ioctl_abi.json
```

Decoded output looks like:

```
[1714390000000] pid=5678 fd=/dev/apusys cmd=0xc0284120 APU_MDW_IOCTL_HANDSHAKE ret=0
[1714390000010] pid=5678 fd=/dev/apusys cmd=0xc0284121 APU_MDW_IOCTL_MEM ret=0
[1714390000050] pid=5678 fd=/dev/apusys cmd=0xc0784122 APU_MDW_IOCTL_CMD ret=0
...

# histogram
   0xc0284120     1  APU_MDW_IOCTL_HANDSHAKE
   0xc0284121    24  APU_MDW_IOCTL_MEM
   0xc0784122     7  APU_MDW_IOCTL_CMD
```

Unknown ioctls show up as `UNKNOWN(type='A', nr=N)` so you can tell the difference between "an ioctl number we know about that someone called" and "an ioctl number we've never seen, on the apusys magic, that maybe represents an ABI revision we haven't transcribed yet".

## What target to attach to

The vendor process that drives the APU on Phone 2a is the NN HAL service. Its name varies by Android version; `scripts/frida-tap.sh` tries three known forms:

```
android.hardware.neuralnetworks-shim-service-mtk     # Android 14+
android.hardware.neuralnetworks@1.3-service.mediatek # Android 12-13
neuralnetworks_hal_service                           # Android 11
```

You can also attach to a specific app:

```sh
frida -U -n com.nothing.camera -l frida/apu_ioctl_tap.js
```

But the camera app calls into the HAL via Binder; you'll only see the HAL's ioctls if you attach to the HAL process itself.

## Sharing traces

If you run this on a different regional/carrier Phone 2a build than the one in `HANDSHAKE.md`, please share the trace via an issue:

- `adb shell getprop ro.build.fingerprint`
- `adb shell uname -a`
- Output of `decode.py`
- First ~100 lines of the raw `.jsonl`

Even just the histogram is useful — it tells us whether other builds use the same cmd numbers or a different midware revision.
