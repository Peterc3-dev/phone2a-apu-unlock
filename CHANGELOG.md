# Changelog

## v0.1.0 — first public release

First handshake on real hardware, ABI verified.

- ABI catalog and Rust transcription complete (4 ioctls, 14 ops)
- Compile-time `static_assert` size guards on `MdwHsArgs`, `MdwMemArgs`, `MdwCmdArgs`, `MdwUtilArgs`
- KernelSU sepolicy module v0.3 grants shell domain rw+ioctl on `/dev/apusys`
- `MDW_HS_IOCTL_OP_BASIC` returns version=3, dev_mask=0x56, mem_mask=0x36, meta_size=32 on the reference device
- All 4 compute engines + 12 MB VLM scratchpad enumerated via per-class handshake
- Frida ioctl tap for vendor-stack cross-checking

### Pre-public iteration history (for context)

- **v0.3 sepolicy module** (current, working) — minimal allow rule on `apusys_device chr_file` + `dir search` + `map`. Confirmed working on the reference device.
- v0.2 (not shipped publicly) — had a missing `dir search` rule which made the SELinux denial fire on the directory-traverse step before `open()` could be checked. Looked like an `apusys_device` denial in audit logs, but the actual block was at `/dev/`.
- v0.1 (not shipped publicly) — had a syntax issue in `sepolicy.rule` that caused the policy to silently fail to load. KernelSU swallowed the parse error.

The pre-v0.3 zips are not included in this repo; they're flawed iterations whose only purpose was to teach us what the right rule looks like.

### Critical bug caught pre-flight

The `static_assert` size guards added during ABI development caught a real bug: the Rust `_pad` filler arrays in `src/uapi.rs` were sized 64/64/192/32 bytes against C unions of 40/40/120/32 bytes. `nix::ioctl_readwrite!` encodes `_IOC_SIZE` from Rust layout, so the encoded cmd values were `0xc0404120 / 0xc0404121 / 0xc0c04122 / 0xc0204123` — wrong size, off-by-one in the size field. Every handshake/mem/cmd ioctl would have returned `-EINVAL` on the phone. Without the asserts, this would have shipped silently and looked like a kernel-side ABI mismatch instead of a Rust struct bug.

Correct cmd values are `0xc0284120 / 0xc0284121 / 0xc0784122 / 0xc0204123`. Anyone who built the binary before that fix landed needs to rebuild.
