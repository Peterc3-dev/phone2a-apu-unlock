# phone2a-apu-unlock

A KernelSU sepolicy module + Rust ABI probe that opens `/dev/apusys` to the shell domain on the Nothing Phone 2a, plus characterization of the MT6886 APU 650's userspace handshake.

**Status:** ABI verified — handshake works on real hardware. DLA submission path: not implemented.

## What this actually is

A tinkerer's foothold on the MediaTek APU 650 NPU on Phone 2a:

- A KernelSU module (`ksu-module/`) that adds a few sepolicy rules so the `shell` domain can `read/write/open/ioctl` on `/dev/apusys` and DMA-buf heap nodes.
- A small Rust binary (`apu650-probe/`) that opens the device, issues the four documented ioctls (`HANDSHAKE`, `MEM`, `CMD`, `UTIL`), and dumps the device's capability bitmask.
- Compile-time `static_assert` guards that verify the Rust struct layout matches the C kernel header. (These caught a real bug in an early iteration where union padding was wrong; without them the encoded `_IOC_SIZE` would have been off and every ioctl would have returned `-EINVAL`.)
- A Frida script (`frida/`) for tapping `ioctl()` calls in the vendor NN HAL service to capture real APU traffic when the closed userspace makes calls (e.g. during a camera Ultra HDR shot).

## What this is NOT

- **Not a NeuroPilot replacement.** MediaTek's NeuroPilot stack (`ncc-tflite` compiler, `libneuron` runtime) exists, is publicly redistributable through the Genio Yocto SDK, and emits `mdla3.0` `.dla` files that target the same accelerator generation as the APU 650.
- **Not the only open route to MTK NPUs.** Both [Google's LiteRT](https://github.com/google-ai-edge/LiteRT/tree/main/litert/vendors/mediatek) and [PyTorch ExecuTorch](https://docs.pytorch.org/executorch/stable/backends-mediatek.html) ship official MediaTek backends. They wrap the same closed runtime; the APU on Phone 2a is reachable from those stacks once the runtime is in place.
- **Not a `.dla` reverse-engineering project.** Nobody has reverse-engineered the format publicly because there's no need to — MediaTek ships the emitter.
- **Not a TFLite-to-DLA compiler.** Just the kernel-driver foothold.

## What the actual gap is

Three observations:

1. The Dimensity 7200 / MT6886 is **not on Google's first-class supported-SoC list** for LiteRT (the list enumerates D7300, D8300, D9000-series; the 7200 is omitted) — see [ai.google.dev/edge/litert/next/mediatek](https://ai.google.dev/edge/litert/next/mediatek).
2. Phone 2a's stock consumer image **does not ship a usable NeuroPilot userspace** for general inference outside the camera/voice paths Nothing's vendor build needs.
3. The publicly available NeuroPilot AOT path is reportedly crippled vs. Google-internal builds — see [LiteRT issue #6462](https://github.com/google-ai-edge/LiteRT/issues/6462) (~153× speedup gap with internal compiler flags).

So this kit gives root-domain access to the device node and a runnable userspace probe on a chip the vendor stack treats as second-class. That's the honest contribution. To go from "handshake works" to "useful inference" you still need either the closed NeuroPilot SDK or somebody to do the multi-month reverse-engineering work to write an open emitter.

## Tested on

Exactly one device, exactly one build. **This has not been validated anywhere else.**

| | |
|---|---|
| Phone | Nothing Phone 2a (codename Pacman, model A142) |
| Build | BP2A.250605.031.A3 (Pacman_B4.0-260225-1817) |
| OS | Android 16 |
| Kernel | 5.15.189-android13-8 |
| Bootloader | unlocked |
| Root | KernelSU-Next v3.2.0 LKM, init_boot_b patched |
| Module | `apu650-unlock-ksu-v0.3.zip` flashed in KSU manager |

## Get to handshake (quickstart)

The full procedure is in `docs/UNLOCK.md`. Summary:

1. **Unlock the bootloader** via Nothing's official flow: <https://help.nothing.tech/hc/en-us/articles/22122594797329>. There's a 7-day waiting period.
2. **Pull the matching `init_boot.img`** from spike0en's archive: <https://github.com/spike0en/Nothing_OTA_Archive>. Match your installed `ro.build.fingerprint`.
3. **Install KernelSU-Next manager** (<https://github.com/rifsxd/KernelSU-Next/releases>, v3.2.0 or newer, **LKM mode**). Patch `init_boot.img` via the manager, flash to the active slot:
   ```sh
   fastboot flash init_boot_b patched_init_boot.img    # or _a, per ro.boot.slot_suffix
   ```
4. **Open the KSU-Next manager once** to bootstrap `ksud`, then in SuperUser tab toggle **Shell** (UID 2000) on.
5. **Build and install the unlock module:**
   ```sh
   cd ksu-module
   zip -r ../apu650-unlock-ksu-v0.3.zip module.prop sepolicy.rule customize.sh post-fs-data.sh
   adb push ../apu650-unlock-ksu-v0.3.zip /sdcard/
   # KSU-Next manager → Modules → Install from storage → pick the zip → reboot
   ```
6. **Build the probe and run:**
   ```sh
   cd apu650-probe
   rustup target add aarch64-linux-android
   # Set linker in ~/.cargo/config.toml first — see apu650-probe/README.md
   cargo build --release --target aarch64-linux-android
   cd ..
   scripts/first-contact.sh
   ```

If everything works:

```
opened /dev/apusys fd=3
version=0x3 dev_mask=0x56 mem_mask=0x36 flags=0x0 meta_size=32
  dev[1] num=2 meta="0x15556"
  dev[2] num=1 meta=""
  ...
```

Compare against `docs/HANDSHAKE.md` to interpret your values.

## Repo layout

```
apu650-probe/      Rust handshake binary + Cargo project
ksu-module/        v0.3 KernelSU sepolicy module sources
frida/             ioctl tap script + decoder
dla/               TFLite test model scaffolding (no compiler — see docs/DLA.md)
abi/               machine-readable ioctl + ops catalog (JSON)
scripts/           first-contact.sh + frida-tap.sh
docs/              UNLOCK, ABI, HANDSHAKE, DLA, FRIDA, ROADMAP
```

## Limitations — read this before assuming it'll work for you

- **Tested on exactly one device.** Different regional/carrier builds *should* be compatible (same SoC, same kernel tree) but this is unverified.
- **Phone 2a Plus (PacmanPro) is untested.** Same SoC family (MT6886), so it probably works the same, but we have no data.
- **DLA submission is not implemented.** You can talk to the driver but can't actually run inference. To do inference today you go through MediaTek's Genio SDK; this repo does not bypass that.
- **The patched `init_boot.img` is device- and build-specific.** It will not transfer to other MTK phones; pull each one's matching boot artifact.
- **Bootloader unlock voids your Nothing warranty.** Obvious, but stated.

## Contributing

Most useful contributions, in order:

1. **Run the handshake on your build and share the output.** Tells us whether the ABI is stable across regional/carrier variants. (Format in `CONTRIBUTING.md`.)
2. **Run the Frida tap during real APU work** (Ultra HDR shot, voice transcription, etc.) and share the captured ioctl trace. Builds a corpus of real subcmd payloads.
3. **Test on Phone 2a Plus** — same SoC family, untouched here.
4. If you have NeuroPilot Genio SDK access: end-to-end `ncc-tflite → MEM(Alloc) → CMD(Run)` round-trip on Phone 2a. Unblocks the CMD path for everyone.

## License

This project is MIT-licensed (see `LICENSE`). The `mdw_ioctl.h` header that `apu650-probe/src/uapi.rs` is derived from is GPL-2.0; the Rust transcription is a structural mirror of the public ABI for interoperability (no kernel code linked).

The **Genio SDK is separately licensed by MediaTek and not redistributable.** Don't push extracted SDK binaries to this repo or any public mirror.

## Credits

- **spike0en** — the [Nothing OTA archive](https://github.com/spike0en/Nothing_OTA_Archive) is the source for build-matched `init_boot.img`s.
- **MediaTek's `mtk-apusys-driver`** ([gitlab](https://gitlab.com/mediatek/aiot/bsp/mtk-apusys-driver)) — the public GPL-2.0 driver tree is the source of truth for the ABI.
- **NothingOSS** — the [Phone 2a kernel sources](https://github.com/Nothing-OSS) confirm the driver tree matches Genio's android13 head byte-for-byte.
- **KernelSU-Next** ([rifsxd/KernelSU-Next](https://github.com/rifsxd/KernelSU-Next)) — LKM-mode root that makes a sepolicy patch possible without rebuilding the kernel.
- **frida** — the obvious tool for tapping ioctls without modifying the vendor stack.
