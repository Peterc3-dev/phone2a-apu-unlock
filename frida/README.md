# Frida ioctl tap

`apu_ioctl_tap.js` is a Frida script that hooks `libc.ioctl()` in any process and logs every ioctl whose target fd resolves (via `/proc/self/fd/<n>`) to a path matching `/dev/(apu|mdla|vpu|mtk_aov|edma|mdw)`. Each record is appended as one JSON line to `/sdcard/apu_ioctl.jsonl`:

```json
{"ts":1714390000000, "tid":1234, "proc":5678, "path":"/dev/apusys",
 "cmd":"0xc0284120", "dir":3, "size":40, "type":"A", "nr":32,
 "arg":"0x7000abcd00", "payload_hex":"...", "ret":0}
```

`decode.py` reads that file plus `../abi/apusys_ioctl_abi.json` and prints a labelled trace plus a histogram.

## When to use it

- **Before running `apu650-probe` on a new device:** confirm that the cmd numbers in `abi/apusys_ioctl_abi.json` match what the vendor stack actually sends. Different MTK midware revisions (1.0 / 1.5 / 2.0) use different ioctl `nr`s.
- **After handshake works but `MEM` or `CMD` returns `-EFAULT`:** capture a real `MdwSubcmdInfo` payload from the NN HAL and compare against `src/uapi.rs`. The struct grew several fields between Genio branches; MT6886 may be on a slightly older revision.
- **To understand what NeuroPilot actually does:** provoke the camera's Ultra HDR pipeline, the gallery's SuperRes, etc., and watch the cmd graph build up.

## Setup

On the host:
```sh
pipx install frida-tools
```

On the phone (rooted):
```sh
# Pick the version matching your host's frida-tools.
adb push frida-server-aarch64 /data/local/tmp/
adb shell su -c 'chmod 0755 /data/local/tmp/frida-server-aarch64'
adb shell su -c 'nohup /data/local/tmp/frida-server-aarch64 >/dev/null 2>&1 &'
```

Then:
```sh
../scripts/frida-tap.sh
# (provoke the APU on the phone — open Camera, take an Ultra HDR shot)
# Ctrl-C when done
adb pull /sdcard/apu_ioctl.jsonl
./decode.py apu_ioctl.jsonl ../abi/apusys_ioctl_abi.json
```

## Sharing traces

If you run this on a Phone 2a with a different region/carrier build than the one in `docs/HANDSHAKE.md`, please share your trace. Open an issue with:

- Build fingerprint: `adb shell getprop ro.build.fingerprint`
- Kernel: `adb shell uname -a`
- Output of `decode.py`
- The first ~100 lines of the `.jsonl` (raw)

That's how we'll learn whether the ABI is stable across regional builds.
