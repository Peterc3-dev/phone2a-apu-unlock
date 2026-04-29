# DLA scaffolding

The `.dla` ("Deep Learning Accelerator") binary format is what the APU 650 actually executes. To run a TFLite model on the NPU you need to compile it to `.dla` first using MediaTek's `ncc-tflite` cross-compiler.

**Status: not yet open.** This directory contains the scaffolding (TFLite test model + compile wrapper), but `ncc-tflite` itself is gated behind MediaTek's developer-portal registration. See `FETCH_SDK.md` for the obtain-it-lawfully procedure and `../docs/DLA.md` for two acquisition paths and the legal/ethical considerations.

## Files

- `make_test_tflite.py` — produces `test_int8.tflite` + reference input/output bins. The smallest model that exercises both Conv2D and a non-trivial activation. Output: 1×8×8×1 int8 → 1×8×8×4 int8.
- `compile_dla.sh` — wraps `ncc-tflite` with `LD_LIBRARY_PATH` shim so the Yocto-built binary runs on a normal desktop Linux distro. Defaults to `--arch=mdla3.0` (best guess for APU 650).
- `extract_genio_sdk.sh` — once you've downloaded the Genio rootfs (tar.gz / squashfs / ext4 image), this pulls only `ncc-tflite` + `neuronrt` + their resolved shared-lib dependencies into `external/genio-sdk/`. Avoids dragging in the whole 1+ GB rootfs.
- `FETCH_SDK.md` — how to obtain the SDK lawfully.

## Workflow once you have the SDK

```sh
pip install --user 'tensorflow==2.15' numpy
python make_test_tflite.py
# produces test_int8.tflite, test_input.bin, test_output_cpu.bin

./compile_dla.sh test_int8.tflite mdla3.0
# produces test_int8.dla
```

That `.dla` is what you'd then push to the phone and submit. Two submission paths:

- **`neuronrt`** (MediaTek's reference runtime, also in the SDK): `adb push` it + the DLA, run on-device. Fastest path to "did the chip compute the right thing." Uses the same ioctls our `apu650-probe` characterizes.
- **`apu650-probe/src/bin/dla-runner.rs`** (this repo): does `MEM(Alloc) → CMD(Run) → poll(fence) → readback` directly via the documented ioctls, no MTK userspace involved. First-run will probably hit `-EFAULT` or `-EINVAL` on subcmd_info field shape since we're guessing some fields; iterate against the Frida tap output to converge. Cleaner license-wise, harder to get right.

## Why we don't ship `ncc-tflite`

The Genio SDK EULA forbids redistribution. We're not going to violate it on a public repo, even if a personal RE defense exists for individual users. Each contributor needs to register and download themselves.
