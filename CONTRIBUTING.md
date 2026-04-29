# Contributing

This is interop research. Tone is matter-of-fact, technically precise, and honest about what works and what doesn't. Marketing language and hype belong elsewhere.

## The single highest-value contribution

**If you have a Phone 2a, run the handshake on your build and share the output.**

Right now we have data from exactly one device on exactly one build (see `docs/HANDSHAKE.md`). Anyone running `apu650-probe` on a different regional or carrier build helps verify whether the ABI is stable across the Phone 2a fleet, or whether different builds ship different midware revisions.

Open an issue with:

- `adb shell getprop ro.build.fingerprint`
- `adb shell uname -a`
- The full output of `apu650-probe` under `su`
- Whether you used the v0.3 sepolicy module unmodified, or had to add anything

If the values match `docs/HANDSHAKE.md`, that's already a useful confirmation. If they don't, the diff is gold.

## Other valuable contributions

In rough order of impact:

1. **Phone 2a Plus / PacmanPro (MT6886)** — same SoC family, completely untested. If you have one, run the handshake.
2. **Frida traces from the vendor stack.** Provoke the camera's Ultra HDR pipeline (or Gallery SuperRes, or any NPU-using feature) while running `scripts/frida-tap.sh`. Pull `/sdcard/apu_ioctl.jsonl`, run `frida/decode.py`, attach the histogram + first ~100 lines of the raw `.jsonl`.
3. **First successful `CMD(Run)`.** Requires Genio SDK access (see `docs/DLA.md`). Whoever lands this first unblocks real inference for everyone else.
4. **DMA-heap sepolicy refinement.** Once `MEM(Alloc)` is exercised on a real device, the dma-buf machinery probably needs additional allow rules in `ksu-module/sepolicy.rule`. Help us figure out which heap names the driver actually pulls from.

## How to send a Frida trace

The trace file can get large. Please:

1. Run `frida/decode.py` and attach the **histogram** to the issue inline.
2. Attach the **raw `.jsonl`** as a gist or a compressed file (gzip is fine).
3. Note which app/process you attached to and what UI action you took.

Don't paste megabytes of hexdump into the issue body.

## Code contributions

- **Match the existing tone.** Comments and docs are matter-of-fact and dense. Cite sources for anything ABI-related (file path + branch + commit if possible).
- **Respect the size guards.** If you change a struct in `apu650-probe/src/uapi.rs`, update both the `_pad` filler and the matching `const_assert_eq!` in the same commit. Don't disable the assertions to "make it build".
- **Run the x86_64 sanity build** (`cargo build --release` in `apu650-probe/`) before submitting. The static_assert guards run there and will catch ABI drift before it reaches the phone.
- **Don't include vendor blobs.** Anything from `/vendor/`, anything extracted from a stock OTA, anything from the Genio SDK — all of that is MediaTek IP. Reference paths and offsets in PRs; don't attach the binaries.

## Things this project will NOT accept

- **Clean-room RE of the `.dla` format.** That's MediaTek's compiler IP. We use the official tooling (under their EULA) to produce DLAs; we don't reimplement the format. PRs that try to do this will be closed.
- **Vendor blob redistribution.** Even sanitized, even "for educational purposes". No.
- **Generic "MTK NPU runtime" scope creep.** This repo is about the APU 650 on Phone 2a specifically. Other MTK SoCs are out of scope.
- **Pre-built APKs / pre-built Rust binaries / committed `target/` directories.** Source-only. Users build for themselves.

## Reporting issues

Issues with reproducible repro steps + full output get answered fastest. "It doesn't work" with no details probably won't.

If you hit a kernel oops or panic, attach `adb shell su -c 'dmesg' | tail -200` from immediately after.
