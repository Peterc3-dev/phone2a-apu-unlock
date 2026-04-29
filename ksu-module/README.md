# apu650-unlock KernelSU module (v0.3)

Grants the Android `shell` domain `read|write|open|ioctl|getattr|map` access to `/dev/apusys` (label `apusys_device`) so a userspace binary running under `adb shell su -c` can drive the APU 650 directly.

## Build the zip

```sh
cd ksu-module
zip -r ../apu650-unlock-ksu-v0.3.zip module.prop sepolicy.rule customize.sh post-fs-data.sh
```

## Install

1. Push the zip:
   ```sh
   adb push apu650-unlock-ksu-v0.3.zip /sdcard/
   ```
2. KernelSU manager → Modules → Install from storage → pick the zip.
3. Reboot.

After reboot, verify:
```sh
adb shell su -c 'ls -lZ /dev/apusys'
# expect:  crw-rw---- ... u:object_r:apusys_device:s0 ...

adb shell su -c 'sesearch -A -s shell -t apusys_device -p open'
# expect:  allow shell apusys_device : chr_file ... open ... ;
```

## Version history

- **v0.3** (current, working revision) — minimal allow rule on `apusys_device chr_file` + `dir search`. Confirmed working on Phone 2a (Pacman, Android 16, kernel 5.15.189-android13-8) with KernelSU-Next v3.2.0 LKM.
- v0.1, v0.2 (not shipped publicly) — earlier iterations had typos in `sepolicy.rule` syntax that made the policy fail to load silently. Don't use them.

## What the rule does

```
allow shell apusys_device chr_file { read write open ioctl getattr map }
allow shell apusys_device dir search
```

The `dir search` line is needed because `/dev/apusys` lives under `/dev/` (a directory in the `device` tclass) — without it, `open()` fails before sepolicy even gets a chance to check `chr_file` permissions.

`map` is included so userspace can `mmap()` the dma-buf fds returned by `MEM(Alloc)` once you start submitting commands. The handshake itself doesn't need `map`, but adding it now means you don't have to reflash later.

## What it does NOT do

- **No DMA-heap allow rules.** Once you reach the `MEM(Alloc)` step you'll likely hit `EACCES` on `/dev/dma_heap/system` or `/dev/dma_heap/mtk_mm-uncached`. The exact tcontext varies by build — run `ls -lZ /dev/dma_heap/` on the device, then add e.g. `allow shell dmabuf_system_heap_device:chr_file rw_file_perms;` to `sepolicy.rule` and re-flash. We didn't pre-add these because guessing wrong can mask real policy bugs.
- **No persistent service hook.** This is a pure sepolicy module — no scripts run at boot.
- **No process-domain transition.** `apu650-probe` runs in the `shell` domain (UID 2000); we don't promote it to `init` or `magisk`.
