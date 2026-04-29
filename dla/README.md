# DLA scaffolding

The `.dla` ("Deep Learning Accelerator") binary format is what the APU 650 actually executes. To run a TFLite model on the NPU you need to compile it to `.dla` first using MediaTek's `ncc-tflite` cross-compiler.

**Status: not yet open.** This directory contains the scaffolding (TFLite test model + compile wrapper), but `ncc-tflite` itself is gated behind MediaTek's developer-portal registration. See `FETCH_SDK.md` for the obtain-it-lawfully procedure and `../docs/DLA.md` for two acquisition paths and the legal/ethical considerations.

## Files

- `make_test_tflite.py` — produces `test_int8.tflite` + reference input/output bins. The smallest model that exercises both Conv2D and a non-trivial activation. Output: 1×8×8×1 int8 → 1×8×8×4 int8.
- `compile_dla.sh` — wraps `ncc-tflite` with `LD_LIBRARY_PATH` shim so the Yocto-built binary runs on a normal desktop Linux distro. Defaults to `--arch=mdla3.0` (best guess for APU 650).
- `FETCH_SDK.md` — how to obtain the SDK lawfully.

## Workflow once you have the SDK

```sh
pip install --user 'tensorflow==2.15' numpy
python make_test_tflite.py
# produces test_int8.tflite, test_input.bin, test_output_cpu.bin

./compile_dla.sh test_int8.tflite mdla3.0
# produces test_int8.dla
```

That `.dla` is what you'd then push to the phone, allocate a cmdbuf for via `MEM(Alloc)`, copy in, and submit via `CMD(Run)`. The runtime piece (`neuronrt`) is in the same SDK; alternatively you can implement the cmdbuf-build + `CMD(Run)` ioctl directly from userspace using the structs in `apu650-probe/src/uapi.rs`.

## Why we don't ship `ncc-tflite`

The Genio SDK EULA forbids redistribution. We're not going to violate it on a public repo, even if a personal RE defense exists for individual users. Each contributor needs to register and download themselves.
