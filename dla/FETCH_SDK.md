# Genio SDK — fetching `ncc-tflite` and `neuronrt`

The DLA toolchain is gated behind MediaTek's i.mediatek.com / `download01.mediatek.com` developer portal — registration with email + product-line check, no anonymous fetch. Only the SDK ships `ncc-tflite` (the cross-compiler that emits `.dla` from `.tflite`) and `neuronrt` (the on-device runtime).

## Steps once you have access

1. Register at <https://i.mediatek.com> (free; "Genio AIoT" product line).
2. Land on <https://download01.mediatek.com/aiot/> and pull the **Genio 700 EVK Yocto image** for v24.0 or newer. Pick the IoT Yocto demo image — it's where `ncc-tflite` lives.
3. The download is a `.tar.gz` (~1 GB). Extract:
   ```sh
   mkdir -p ~/genio-sdk
   tar xzf genio-700-demo-image-*.tar.gz -C ~/genio-sdk
   ```
4. The binary is at `~/genio-sdk/usr/bin/ncc-tflite` and shared libs at `~/genio-sdk/usr/lib/`.
5. Verify on your host:
   ```sh
   LD_LIBRARY_PATH=~/genio-sdk/usr/lib \
       ~/genio-sdk/usr/bin/ncc-tflite --help | head -20
   ```
   May need `patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 ncc-tflite` for non-Yocto rootfs.

6. Then:
   ```sh
   cd dla
   python make_test_tflite.py   # produces test_int8.tflite + test_input/output bins
   ./compile_dla.sh test_int8.tflite mdla3.0   # produces test_int8.dla
   ```

## Arch flag for MT6886

**Best guess: `mdla3.0`.** Genio SDK supports `mdla1.5`, `mdla2.0`, `mdla3.0`, `mdla3.5`, plus `mvpu` and `vpu`. APU 650 is the Dimensity 7200 generation (2022); MDLA 3.0 is the most likely match. If `ncc-tflite` rejects it with `NEURON_BAD_DATA` at compile time, fall back to `mdla2.0` or `mdla3.5` and document which the phone accepts.

## License caveat

The Genio SDK EULA forbids redistribution. Personal RE on hardware you own (Phone 2a) is interop work and defensible under EU 2009/24/EC Art. 6 / US 17 USC 1201(f). **Do not push the SDK or extracted binaries to a public repo.** Keep artifacts out of git.

## License-free alternative (slower path)

If MediaTek registration ever stops working, the Yocto build is reproducible from `https://gitlab.com/mediatek/aiot/bsp/manifest` — clone, `repo init`, `bitbake genio-700-demo-image`. Takes several hours and ~80 GB disk. Not recommended unless the registered download path collapses.
