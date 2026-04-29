# Roadmap

## Done

- [x] Identify the kernel-side ABI source (Genio android13 `mtk-apusys-driver`)
- [x] Verify byte-for-byte that NothingOSS Pacman kernel uses the same `mdw_ioctl.h`
- [x] Transcribe the four ioctls + 14 ops to Rust (`apu650-probe/src/uapi.rs`)
- [x] Produce machine-readable ABI catalog (`abi/apusys_ioctl_abi.json`)
- [x] Catch the union-size drift bug with compile-time `static_assert` guards
- [x] Bootloader unlock walkthrough verified on real Phone 2a (Pacman, A142)
- [x] KernelSU-Next LKM mode confirmed working with patched `init_boot`
- [x] sepolicy module v0.3 grants shell domain rw+ioctl on `/dev/apusys`
- [x] First handshake: `MDW_HS_IOCTL_OP_BASIC` returns version=3, `meta_size=32` ✓
- [x] Per-class device enumeration: 4 engines (MDLA×2, VPU×1, EDMA×1, AOV×1)
- [x] Per-pool memory enumeration: VLM 12 MB scratchpad confirmed
- [x] Frida ioctl tap script + decoder

## Next

- [ ] **Get a `.dla` artifact.** Either via Genio SDK registration (path 1) or Yocto build (path 2). See `DLA.md`. **This is the biggest single blocker.**
- [ ] **First `MEM(Alloc)` round-trip.** Allocate a small cmdbuf, mmap it, write/read pattern, free. No firmware involvement; just driver-side dma-buf machinery. Easy win to verify the next ioctl works.
- [ ] **First `CMD(Run)` with the smallest possible DLA.** A 1×8×8×1 → 1×8×8×4 INT8 Conv2D+ReLU model is staged at `dla/make_test_tflite.py`. Compile, push, submit, poll fence, read output, bit-compare against CPU reference.
- [ ] **DMA-heap sepolicy expansion.** Once `MEM(Alloc)` lands, the dma-buf machinery probably tries to allocate from `/dev/dma_heap/system` or `/dev/dma_heap/mtk_mm-uncached`. Add the right allow rule to `ksu-module/sepolicy.rule` v0.4.
- [ ] **Capture vendor cmdbuf payloads** via Frida tap on `android.hardware.neuralnetworks-shim-service-mtk` while provoking an Ultra HDR shot. Compare struct layouts against `uapi.rs`; this is the cheapest way to detect any `MdwSubcmdInfo` shrinkage on MT6886.

## Where contribution is most valuable

In rough order of impact:

1. **Run the handshake on a different regional/carrier Phone 2a build** and share the captured values. This is what tells us whether the ABI in `uapi.rs` is stable across regions.
2. **Test on Phone 2a Plus (PacmanPro).** Same SoC family; *probably* compatible, but completely untested.
3. **Provide a Frida trace from a working NeuroPilot pipeline.** Camera Ultra HDR or Gallery SuperRes are the easiest to provoke. Even just the histogram + first 100 lines is useful.
4. **First successful `CMD(Run)`.** Requires Genio SDK access. Whoever lands this first opens the rest of the pipeline for everyone.
5. **DMA-heap allow rules.** Once we know which heap the driver pulls from on a real Phone 2a, finalize a v0.4 sepolicy module that doesn't require trial-and-error.

## What this project is NOT going to do

- **Clean-room RE the DLA format.** That's MediaTek's compiler IP. We use the official tools (under EULA) to produce DLAs; we don't reimplement them.
- **Ship NeuroPilot binaries.** The vendor blobs (`/vendor/lib/libneuron*.so`, `/vendor/bin/neuronrt`) are MediaTek IP. Don't redistribute.
- **Support phones outside the MT6886 / Genio-android13 family.** Other MTK SoCs have different ABIs; this project is scoped to APU 650 specifically.
- **Become a generic "MTK NPU runtime".** Out of scope. The point is to verify the ABI on Phone 2a and unblock on-device inference for that one device class.

## What's beyond this repo

If `CMD(Run)` works end-to-end on a `.dla`, the obvious next layer is a TFLite delegate that targets `/dev/apusys` directly — i.e. an open-source replacement for MediaTek's NN HAL. That would let a normal Android app target the APU without going through the vendor stack.

We're not building that here. This repo stops at "the kernel ABI is open and works"; the userspace runtime layer is a separate project.
