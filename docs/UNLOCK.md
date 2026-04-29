# Unlock walkthrough — bootloader → KernelSU → module → handshake

This is the end-to-end procedure that worked on a Phone 2a (Pacman, A142) running build BP2A.250605.031.A3 (Pacman_B4.0-260225-1817), Android 16, kernel 5.15.189-android13-8.

It will probably work on other regional/carrier builds with minor adjustments, but **it has only been verified on one device**. If you run it and hit a wall, please open an issue.

## 0. Prerequisites

- Nothing Phone 2a (any region — should be the same; only A142 confirmed)
- A USB-C cable + a Linux/macOS/Windows host with `adb` and `fastboot`
- Patience: Nothing's bootloader unlock has a 7-day waiting period

## 1. Unlock the bootloader (Nothing's official flow)

Follow the official Nothing flow: <https://help.nothing.tech/hc/en-us/articles/22122594797329>. Summary:

1. Settings → About phone → tap Build number 7 times → Developer mode on.
2. Developer options → enable **OEM unlocking** and **USB debugging**.
3. Reboot to fastboot: `adb reboot bootloader`.
4. `fastboot oem unlock` — this issues a 7-day waiting period.
5. Wait 7 days. (Yes, really. The phone enforces this server-side.)
6. After 7 days, repeat steps 3–4. The unlock will go through; the phone will factory-reset.

## 2. Pull the matching boot.img

You need the exact `boot.img` that matches your installed build. The cleanest source is **spike0en's archive**: <https://github.com/spike0en/Nothing_OTA_Archive>.

1. Find your installed build:
   ```sh
   adb shell getprop ro.build.fingerprint
   # e.g.  Nothing/Pacman/Pacman:16/BP2A.250605.031.A3/...
   ```
2. Download the matching firmware archive from spike0en.
3. Extract `init_boot.img` (Phone 2a uses `init_boot` for ramdisk patching, **not** `boot.img` — important).

> **Why `init_boot` and not `boot`?** Phone 2a is a GKI-style Android 13+ device where the kernel lives in `boot.img` and the (small) ramdisk that runs `init` lives in `init_boot.img`. KernelSU-Next LKM patches `init_boot` to inject its loader; the kernel itself is untouched. This is what makes the LKM mode possible without rebuilding the kernel.

## 3. Install KernelSU-Next manager + patch init_boot

1. Download the KernelSU-Next manager APK from <https://github.com/rifsxd/KernelSU-Next/releases>. We used **v3.2.0** in LKM mode. (Vanilla KernelSU does not support LKM cleanly on this kernel — use the Next fork.)
2. Install the APK: `adb install KernelSU-Next_v3.2.0_xxxxx.apk`.
3. Open the manager. It will show "Working mode: not installed".
4. Tap the install icon (top-right). Select **Patch init_boot.img**. Pick the `init_boot.img` you extracted in step 2.
5. The manager produces `KernelSU_Next_init_boot_<timestamp>.img` somewhere in the Files app — typically `Internal storage/Download/`. Pull it:
   ```sh
   adb pull /sdcard/Download/KernelSU_Next_init_boot_<timestamp>.img patched_init_boot.img
   ```

## 4. Flash the patched init_boot to the active slot

Find the active slot:
```sh
adb shell getprop ro.boot.slot_suffix
# e.g.  _b
```

Flash to the matching slot:
```sh
adb reboot bootloader
fastboot flash init_boot_b patched_init_boot.img   # or init_boot_a if your slot is _a
fastboot reboot
```

## 5. Bootstrap KSU and grant Shell SU

After reboot, open the KernelSU-Next manager **once**. This is the bootstrap step — the manager copies `ksud` into place and does first-time setup. If you skip this, modules won't install.

Then in the manager → **SuperUser** tab → find **Shell** (UID 2000) → toggle it **on**. This is what lets `adb shell su -c '...'` actually return uid=0.

Verify:
```sh
adb shell su -c id
# expect:  uid=0(root) gid=0(root) ...
```

## 6. Install the APU 650 unlock module

Build the v0.3 zip (from this repo's root):
```sh
cd ksu-module
zip -r ../apu650-unlock-ksu-v0.3.zip module.prop sepolicy.rule customize.sh post-fs-data.sh
cd ..
```

Push and install:
```sh
adb push apu650-unlock-ksu-v0.3.zip /sdcard/
# In KSU-Next manager → Modules → Install from storage → pick the zip
adb reboot
```

Verify after reboot:
```sh
adb shell su -c 'sesearch -A -s shell -t apusys_device -p open'
# expect:  allow shell apusys_device : chr_file ... open ... ;
```

If `sesearch` isn't on the device, `first-contact.sh` will catch the policy state at `open()` time anyway.

## 7. Build and run apu650-probe

You need the Android NDK. Install it (any reasonably recent NDK works; we used r26d). Set up `~/.cargo/config.toml`:

```toml
[target.aarch64-linux-android]
linker = "/path/to/Android/Sdk/ndk/r26d/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android29-clang"
```

Then:
```sh
rustup target add aarch64-linux-android
cd apu650-probe
cargo build --release --target aarch64-linux-android
cd ..

scripts/first-contact.sh
```

You should see output like:
```
opened /dev/apusys fd=3
version=0x3 dev_mask=0x56 mem_mask=0x36 flags=0x0 meta_size=32
  dev[1] num=2 meta="0x15556"
  ...
```

If you see this, **you're at the same handshake point we are.** See `HANDSHAKE.md` for what each field means and how to compare against our reference values.

## Failure modes

| Symptom | Likely cause | Fix |
|---|---|---|
| `Permission denied` opening `/dev/apusys` | sepolicy not active | Confirm the KSU module is enabled in the manager, reboot |
| `No such file or directory` | Wrong device node — older midware | `adb shell ls /dev/apu*` to see what's there; might need to adjust `DEV` const in `apu650-probe/src/main.rs` |
| `-EINVAL` / `Inappropriate ioctl` | Cmd-number mismatch (struct sizes drifted) | Capture real ioctls with the Frida tap, compare against `abi/apusys_ioctl_abi.json` |
| `-EFAULT` / `Bad address` | Struct size mismatch in payload | The static_assert size guards in `src/uapi.rs` should have caught it at compile time. If they didn't, the kernel struct grew/shrunk on your build — re-derive from a Frida trace |
| `id` returns `uid=2000` instead of `uid=0` | Forgot to toggle Shell SU on in KSU manager | Open the manager, SuperUser tab, toggle Shell |
| KSU manager says "Working mode: not installed" after reboot | Patched img flashed to wrong slot | Re-check `ro.boot.slot_suffix`, re-flash to the right `init_boot_a` or `init_boot_b` |
