# apu650-probe

A small Rust binary that opens `/dev/apusys` on a rooted Nothing Phone 2a (or any MT6886/Genio-android13 device) and issues the three sub-ops of `APU_MDW_IOCTL_HANDSHAKE` to enumerate the APU 650's capabilities — uapi version, device-class bitmask (MDLA / VPU / EDMA / AOV), memory-pool bitmask, and the per-class metadata blob.

It does **not** submit any commands. It is a capability probe — the safe first step. The point is to confirm that the ABI as transcribed in `src/uapi.rs` matches what the kernel actually speaks before any `MEM(Alloc)` or `CMD(Run)` is attempted.

## Build

### x86_64 Linux host (sanity build, won't run on the phone)

```sh
cargo build --release
```

This is useful because the `static_assert` size guards in `src/uapi.rs` run at compile time and catch struct-layout drift before you push anything.

### aarch64 Android target (the binary you actually push)

You need the Android NDK. Set up `~/.cargo/config.toml` with the linker:

```toml
[target.aarch64-linux-android]
linker = "/path/to/Android/Sdk/ndk/r26d/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android29-clang"
```

Then:

```sh
rustup target add aarch64-linux-android
cargo build --release --target aarch64-linux-android
```

Output binary: `target/aarch64-linux-android/release/apu650-probe` (~300 KB stripped ELF).

## Run

```sh
adb push target/aarch64-linux-android/release/apu650-probe /data/local/tmp/
adb shell su -c '/data/local/tmp/apu650-probe'
```

Or use `../scripts/first-contact.sh` which does push + sanity checks + run in one go.

## What success looks like

```
opened /dev/apusys fd=3
version=0x3 dev_mask=0x56 mem_mask=0x36 flags=0x0 meta_size=32
  dev[1] num=2 meta="0x15556"
  dev[2] num=1 meta=""
  dev[4] num=1 meta="$"
  dev[6] num=1 meta=""
  mem[1] start=0x2000000 size=0xc00000
  mem[2] start=0x0 size=0x0
  mem[4] start=0x0 size=0x0
  mem[5] start=0x0 size=0x100000
```

The actual values are documented in `../docs/HANDSHAKE.md`. `meta_size=32` is the strongest single confirmation that the ABI matches `MDW_DEV_META_SIZE` in `src/uapi.rs`.

## The static_assert story

`nix::ioctl_readwrite!` encodes `_IOC_SIZE` into the cmd number from the **Rust** struct layout. If the Rust unions are sized differently from the C originals, the encoded ioctl numbers shift and the kernel returns `-EINVAL` on every call.

An early version of `src/uapi.rs` had `_pad: [u8; 64]` filler arrays where the C unions were 40 bytes. The four `const_assert_eq!` lines at the bottom of `src/uapi.rs` caught it at compile time:

```rust
const_assert_eq!(std::mem::size_of::<MdwHsArgs>(), 40);
const_assert_eq!(std::mem::size_of::<MdwMemArgs>(), 40);
const_assert_eq!(std::mem::size_of::<MdwCmdArgs>(), 120);
const_assert_eq!(std::mem::size_of::<MdwUtilArgs>(), 32);
```

Without those, the binary would have built clean and silently sent wrong cmd numbers. Keep these guards. If you change a struct, update the corresponding `_pad` and the assertion together.

## See also

- `GOTCHAS.txt` — ABI drift hazards on MT6886 vs MT8195
- `../docs/ABI.md` — full ioctl + ops catalog
- `../docs/HANDSHAKE.md` — captured handshake values from a real Phone 2a
