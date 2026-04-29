# DLA — what it is, how to get one, current status

The APU 650 doesn't execute TFLite or ONNX directly. It executes a binary format MediaTek calls **DLA** ("Deep Learning Accelerator binary"). Compiling a model to `.dla` is the gate between "we can talk to the driver" and "we can actually run inference".

## Status

**Not yet open.** Handshake works (see `HANDSHAKE.md`). `MEM(Alloc)` + `CMD(Run)` ioctls are reachable through the patched sepolicy but have not been exercised because we don't yet have a `.dla` artifact to submit.

## What you need

Two binaries, both shipped only by the **Genio SDK**:

- `ncc-tflite` — the cross-compiler. Lives in the Genio Yocto BSP at `/usr/bin/ncc-tflite`. Takes a `.tflite` model + an `--arch=` flag (`mdla1.5` / `mdla2.0` / `mdla3.0` / `mdla3.5` / `mvpu` / `vpu`) and emits `.dla`.
- `neuronrt` — the on-device runtime that knows how to load a `.dla`, set up its buffers, and submit it to `/dev/apusys`. Shipped in the same Yocto image at `/usr/bin/neuronrt`.

Best guess for APU 650 architecture flag is `mdla3.0` — the Dimensity 7200 generation maps to MDLA 3.0 in the Genio matrix. If `ncc-tflite` returns `NEURON_BAD_DATA`, fall back to `mdla2.0` or `mdla3.5`.

## Acquisition path 1 (recommended): MediaTek developer portal

This is the lawful, documented path. Pros: legitimate, fast, free. Cons: requires registration and a working email.

Step-by-step in `dla/FETCH_SDK.md`. Summary:

1. Register at <https://i.mediatek.com> (free, "Genio AIoT" product line).
2. Download the **Genio 700 EVK Yocto image** from `download01.mediatek.com/aiot/`.
3. Extract `/usr/bin/ncc-tflite` + `/usr/lib/libneuron*.so`.
4. Possibly run `patchelf` to fix the interpreter for non-Yocto rootfs.

The Genio SDK EULA forbids redistribution. **Do not commit extracted binaries to this repo or any public mirror.** Keep them local.

## Acquisition path 2 (slower): build Yocto from source

The Genio Yocto BSP manifest is fully open: <https://gitlab.com/mediatek/aiot/bsp/manifest>.

```sh
mkdir genio-bsp && cd genio-bsp
repo init -u https://gitlab.com/mediatek/aiot/bsp/manifest
repo sync
bitbake genio-700-demo-image
```

Takes several hours and ~80 GB disk. Use this only if the registration path collapses.

The ABI itself is open (the `mtk-apusys-driver` gitlab is GPL-2.0, no NDA). What's restricted is the closed-source compiler that emits the `.dla` format.

## Acquisition path 3 (long shot): RE the format

The `.dla` format is opaque. Two open analyses exist:

- An old reverse-engineering attempt for Helio P-series: very partial.
- The `apusys_rv` user-mode firmware in the Genio BSP exposes some struct definitions for the cmdbuf shape, but not the ISA itself.

A clean-room RE of the ISA is **not** what this project is about. The driver-side ABI is open; the compiler-side ISA is MediaTek's IP and we're not going to clean-room it. If you want to run inference on your own Phone 2a, register for the SDK.

## After you have a `.dla`

Rough sketch of what end-to-end inference looks like (uses ioctls from `apu650-probe/src/uapi.rs`):

1. `MEM(Alloc)` a buffer of the right size and direction for each cmdbuf (input tensor, weights, output tensor). Each call returns a `u64 handle` that's actually a dma-buf fd. `mmap()` it to get a CPU view; copy in your tensor data.
2. Build a `MdwSubcmdInfo[]` referencing those handles by cmdbuf descriptor. The subcmd graph encodes the model layer-by-layer with adjacency-matrix dependencies.
3. `CMD(Run)` with a `MdwCmdInExec`. Reads back an `out_fence` fd from `MdwCmdOutExec.fence`.
4. `poll(out_fence, POLLIN)` until ready.
5. `MEM(Invalidate)` on the output cmdbuf so the CPU sees the freshly-written data.
6. Read the output tensor back from the mmap.

There's nothing exotic here — it's the standard dma-buf+fence pattern that DRM/V4L2 use. The work is in building the cmdbuf payloads in the format the on-device firmware expects, which is what `ncc-tflite` does for you.

## Help wanted

If you have access to the Genio SDK and run `ncc-tflite` on a Phone 2a, please share:

- Which `--arch=` flag was accepted (`mdla3.0` confirmed? something else?)
- A successful end-to-end submit + result. Even just the printed ioctl trace.
- Any patches needed to `apu650-probe/src/uapi.rs` to make `CMD(Run)` accept your cmdbuf graph.

That's the contribution that would unblock real inference for everyone else.
