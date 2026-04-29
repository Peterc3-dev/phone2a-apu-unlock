# APU 650 ABI

Source of truth: <https://gitlab.com/mediatek/aiot/bsp/mtk-apusys-driver>, branch `android13`, path `midware/2.0/mdw_ioctl.h`. The relevant kernel sources live in this same tree under `midware/2.0/mdw_drv.c`, `mdw_hs.c`, `mdw_mem.c`, `mdw_cmd.c`.

We verified byte-for-byte that the NothingOSS Pacman kernel (`android_kernel_5.15_nothing_mt6886`, branch `mt6886/Pacman/t`) carries the same `mdw_ioctl.h`. The Phone 2a's running kernel is built from this tree, so the Genio android13 ABI applies.

## Device node

- Path: `/dev/apusys`
- SELinux label: `u:object_r:apusys_device:s0`
- Magic: `'A'` (0x41) — `APUSYS_MAGICNO`

## The four ioctls

| Name | nr | dir | size | encoded cmd | purpose |
|---|---|---|---|---|---|
| `APU_MDW_IOCTL_HANDSHAKE` | 32 | RW | 40 | `0xc0284120` | Capability discovery — uapi version, dev/mem bitmasks, per-class metadata |
| `APU_MDW_IOCTL_MEM` | 33 | RW | 40 | `0xc0284121` | Allocate / free / map / unmap / flush / invalidate dma-buf-backed buffers |
| `APU_MDW_IOCTL_CMD` | 34 | RW | 120 | `0xc0784122` | Submit / re-submit / delete a command graph |
| `APU_MDW_IOCTL_UTIL` | 35 | RW | 32 | `0xc0204123` | Power hint (force-on a device clock domain) + opaque user-cmd passthrough |

The "size" column is `_IOC_SIZE` from the macro — the byte length of the union arg. **These sizes were obtained via `gcc sizeof()` on the actual header**, not by hand-counting struct fields. That distinction matters; see "The size mismatch story" below.

## The full ops table

### `APU_MDW_IOCTL_HANDSHAKE` (cmd 32)

| op | name | reply contents |
|---|---|---|
| 0 | `MDW_HS_IOCTL_OP_BASIC` | `version: u64`, `dev_bitmask: u64`, `mem_bitmask: u64`, `flags: u64`, `meta_size: u32`, `reserved: u32` |
| 1 | `MDW_HS_IOCTL_OP_DEV` | For input `type: u32`: `num: u32` (cores of that type), `meta: u8[32]` (per-class metadata blob) |
| 2 | `MDW_HS_IOCTL_OP_MEM` | For input `type: u32`: `start: u64` (device VA base), `size: u32` |

The `version` field is **not** the kernel version or the driver version — it's `mdev->uapi_ver`, an internal user-ABI revision stamp. On the Genio android13 head it's `3`.

`meta_size` is always `MDW_DEV_META_SIZE = 32` on this revision. If your handshake reply has a different value, **stop** — you're on a different midware revision and the rest of `uapi.rs` may not apply.

### `APU_MDW_IOCTL_MEM` (cmd 33)

| op | name | purpose |
|---|---|---|
| 0 | `MDW_MEM_IOCTL_ALLOC` | Allocate dma-buf-backed buffer; returns `u64 handle` (a dma-buf fd disguised as a handle) |
| 1 | `MDW_MEM_IOCTL_FREE` | Release a handle |
| 2 | `MDW_MEM_IOCTL_MAP` | Import an external dma-buf fd into the apusys context (returns `device_va`) |
| 3 | `MDW_MEM_IOCTL_UNMAP` | Release a previously-mapped external buffer |
| 4 | `MDW_MEM_IOCTL_FLUSH` | CPU→device cache sync |
| 5 | `MDW_MEM_IOCTL_INVALIDATE` | device→CPU cache sync |

`MdwMemType`: `Main=0, Vlm=1, Local=2, System=3, SystemIsp=4, SystemApu=5`.
`F_MDW_MEM_*` flags: `CACHEABLE = 1<<0`, `32BIT = 1<<1`, `HIGHADDR = 1<<2`.

The handle returned by `ALLOC` is a `u64`, but it's actually a dma-buf fd internally. Userspace `mmap()` it (using the same fd as a file descriptor — yes, the API conflates the two) to get a CPU view.

### `APU_MDW_IOCTL_CMD` (cmd 34)

| op | name | purpose |
|---|---|---|
| 0 | `MDW_CMD_IOCTL_RUN` | Submit a fresh command graph (subcmds + links + cmdbufs) |
| 1 | `MDW_CMD_IOCTL_RUN_STALE` | Re-submit a previously-built cmd by id |
| 2 | `MDW_CMD_IOCTL_DEL` | Delete/cancel by id |

Cmdbuf direction enum: `MDW_CB_BIDIRECTIONAL=0, MDW_CB_IN=1, MDW_CB_OUT=2`.

`MdwCmdInExec` is a fat 16-field struct that includes pointer-shaped fields for `subcmd_infos` (array of `MdwSubcmdInfo`), `adj_matrix` (adjacency-matrix bytes for the subcmd dependency graph), `fence` (in/out dma-fence fd), `exec_infos` (array of timing/return-code structs), and `links` (array of `MdwSubcmdLinkV1`).

`MdwSubcmdInfo` is the field most likely to drift between branches — it grew several boost/affinity/turbo fields between Genio versions. **MT6886 may carry a shorter struct.** The first sign of drift would be `-EFAULT` on `RUN`.

### `APU_MDW_IOCTL_UTIL` (cmd 35)

| op | name | purpose |
|---|---|---|
| 0 | `MDW_UTIL_IOCTL_SETPOWER` | Force-on a device clock domain (power hint) |
| 1 | `MDW_UTIL_IOCTL_UCMD` | Pass-through: arbitrary opaque user-cmd to a device type |

## The size-mismatch story

`nix::ioctl_readwrite!` in Rust encodes `_IOC_SIZE` from the **Rust** struct layout. If your `repr(C) union` has an oversized `_pad` array, the encoded ioctl number silently shifts and the kernel rejects with `-EINVAL` on every call.

An early version of `apu650-probe/src/uapi.rs` had `_pad: [u8; 64]` filler bytes where the C unions were 40 bytes. The encoded cmd was `0xc0404120` instead of the correct `0xc0284120` — same direction, same magic, same nr, **wrong size**. Every handshake/mem/cmd ioctl would have returned `-EINVAL` on the phone.

The fix: `static_assert` the four union sizes against gcc-computed sizes from the C header:

```rust
const_assert_eq!(std::mem::size_of::<MdwHsArgs>(), 40);
const_assert_eq!(std::mem::size_of::<MdwMemArgs>(), 40);
const_assert_eq!(std::mem::size_of::<MdwCmdArgs>(), 120);
const_assert_eq!(std::mem::size_of::<MdwUtilArgs>(), 32);
```

These four lines turn an ABI-drift bug into a compile-time failure. Keep them. If you change a struct, update the corresponding `_pad` filler **and** the assertion in the same commit.

## Authoritative catalog

The machine-readable version of all of the above is at `abi/apusys_ioctl_abi.json`. `frida/decode.py` joins captured ioctl traces against this JSON.
