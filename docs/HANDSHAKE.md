# First handshake — captured values

This is what `apu650-probe` printed on a Phone 2a (Pacman, A142) running BP2A.250605.031.A3, Android 16, kernel 5.15.189-android13-8, on 2026-04-29.

If you run the probe on your own Phone 2a and see different values, please open an issue with the diff — that's how we'll learn whether the ABI is stable across regional/carrier builds.

## BASIC reply

```
version=0x3 dev_mask=0x56 mem_mask=0x36 flags=0x0 meta_size=32
```

| field | observed | interpretation |
|---|---|---|
| `version` | `0x3` | `mdev->uapi_ver = 3`. Matches Genio android13 HEAD. |
| `dev_bitmask` | `0x56` (binary `01010110`) | bits 1, 2, 4, 6 set → 4 device classes exposed |
| `mem_bitmask` | `0x36` (binary `00110110`) | bits 1, 2, 4, 5 set → 4 memory pools exposed |
| `flags` | `0` | reserved, echoed back |
| `meta_size` | `32` | matches `MDW_DEV_META_SIZE` constant — strongest single confirmation that the ABI is the one in `uapi.rs` |

## DEV replies (one per dev_bitmask bit)

| bit | num | meta (32-byte, ASCII-trimmed) | likely role |
|---|---|---|---|
| 1 | 2 | `"0x15556"` | MDLA (CNN accelerator) — 2 cores |
| 2 | 1 | `""` | VPU/MVPU (vision processor) |
| 4 | 1 | `"$"` (0x24) | EDMA (DMA controller) |
| 6 | 1 | `""` | AOV (always-on vision) |

The `num` field is the number of cores of that class. The `meta` blob is opaque per-class metadata — its meaning varies by class. The `0x15556` string in MDLA's meta is probably an architecture/revision identifier; we don't know how to decode it yet.

The bit-to-class mapping (1=MDLA, 2=VPU, 4=EDMA, 6=AOV) is inferred from context — we matched bit indices against `MDW_DEV_*` enums in the kernel source. It's not 100% confirmed; if you have the SDK, `neuronrt --info` would print authoritative names.

## MEM replies (one per mem_bitmask bit)

| bit | start | size | likely role |
|---|---|---|---|
| 1 | `0x2000000` | `0xC00000` | VLM scratchpad — 12 MB, mapped at +32 MB |
| 2 | `0x0` | `0x0` | (zeroed — possibly Main DRAM, allocated lazily) |
| 4 | `0x0` | `0x0` | (zeroed — possibly Local) |
| 5 | `0x0` | `0x100000` | 1 MB — pool of unknown purpose |

Bit 1 (VLM, "Vector Local Memory") is the on-die scratchpad — this is the small, fast SRAM the MDLA/VPU use as their working set. 12 MB is plausible for the APU 650.

The zero-base/zero-size pools are normal: those pool types are allocated lazily on first `MEM(Alloc)` call rather than reserved at boot. They show up in the bitmask because the driver is willing to serve allocations from them.

## What this tells us

1. **The ABI in `apu650-probe/src/uapi.rs` is correct on this device.** `meta_size=32` is the canary; if it were anything else, every subsequent assumption in `uapi.rs` would be suspect.
2. **The APU 650 exposes four compute engines.** MDLA is the heavy lifter (CNNs); VPU does signal/vision pre-processing; EDMA is the buffer mover; AOV is the low-power always-on path.
3. **There's a 12 MB VLM scratchpad.** That's the on-die memory budget for tile-based inference. Big enough for most mobile-scale models; not big enough to hold an entire 7B LLM weight in one tile.
4. **`MEM(Alloc)` and `CMD(Run)` are wired but untested.** All four ioctls are reachable from userspace through the patched sepolicy; only handshake has been issued.

## Variations to expect

- **Different regional builds.** Same SoC and same Android version *should* give the same `version` and `meta_size`. The bitmasks could differ if Nothing flipped a build flag in some regions to disable AOV (bit 6) or VPU (bit 2).
- **OTA updates.** A future Phone 2a OTA could ship a newer midware (uapi_ver=4) with shifted struct layouts. The `static_assert` size guards in `uapi.rs` won't catch that — the kernel would; you'd see `-EINVAL` on first call. Re-derive structs from a Frida trace.
- **Phone 2a Plus (PacmanPro, MT6886+).** Probably similar; **untested**. Don't assume.
- **Other MT6886/MT6985 phones.** The kernel ABI is shared; the userspace device-tree may differ. Worth trying.
