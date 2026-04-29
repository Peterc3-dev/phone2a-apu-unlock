// Verbatim transcription of mdw_ioctl.h from
// https://gitlab.com/mediatek/aiot/bsp/mtk-apusys-driver
// branch: android13, path: midware/2.0/mdw_ioctl.h
// commit: 57ccb1ae (HEAD of android13 at fetch time)
//
// Original header is GPL-2.0; this is a Rust translation of the
// kernel<->userspace ABI for purposes of probing the device. No
// kernel code is linked.
//
// The Flora Fu MT8192 v4 series
// (patchwork.kernel.org/series/593809) is clock + power-domain +
// DT-binding plumbing only. It does NOT define the user ABI;
// the only public source for that is the gitlab driver above.

#![allow(non_camel_case_types, dead_code)]

use nix::ioctl_readwrite;

pub const APUSYS_MAGICNO: u8 = b'A';

/// MDW_DEV_META_SIZE — fixed-size metadata blob attached to each
/// per-device handshake reply. Driver memcpy's mdev->dinfos[type]->meta
/// into args->out.dev.meta on MDW_HS_IOCTL_OP_DEV.
pub const MDW_DEV_META_SIZE: usize = 32;

// ---- handshake (cmd 32) ----------------------------------------

#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum MdwHsOp {
    Basic = 0,
    Dev = 1,
    Mem = 2,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MdwHsInDev {
    pub r#type: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MdwHsInMem {
    pub r#type: u32,
}

#[repr(C)]
pub union MdwHsInPayload {
    pub dev: MdwHsInDev,
    pub mem: MdwHsInMem,
    _pad: [u8; 8],
}

#[repr(C)]
pub struct MdwHsIn {
    /// One of MdwHsOp. Driver dispatches in mdw_hs_ioctl().
    pub op: u32,
    /// Reserved/unused on input by mdw_hs_ioctl; preserved as flags.
    pub flags: u64,
    pub payload: MdwHsInPayload,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct MdwHsOutBasic {
    /// mdev->uapi_ver — driver-internal user ABI version stamp.
    pub version: u64,
    /// Bitmask of MDW_DEV_* the driver advertises (DSP/DLA/DMA/...).
    pub dev_bitmask: u64,
    /// Bitmask of MdwMemType the driver advertises.
    pub mem_bitmask: u64,
    /// Reserved flags echoed by the driver.
    pub flags: u64,
    /// Always MDW_DEV_META_SIZE (32) on this revision.
    pub meta_size: u32,
    pub reserved: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MdwHsOutDev {
    /// Echoes input type.
    pub r#type: u32,
    /// dinfos[type]->num — number of cores of that device type.
    pub num: u32,
    /// dinfos[type]->meta — opaque per-device metadata blob.
    pub meta: [u8; MDW_DEV_META_SIZE],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct MdwHsOutMem {
    pub r#type: u32,
    pub reserved: u32,
    /// minfos[type].device_va — base device VA of that mem region.
    pub start: u64,
    /// minfos[type].dva_size — size in bytes.
    pub size: u32,
}

#[repr(C)]
pub union MdwHsOut {
    pub basic: MdwHsOutBasic,
    pub dev: MdwHsOutDev,
    pub mem: MdwHsOutMem,
}

#[repr(C)]
pub union MdwHsArgs {
    pub r#in: std::mem::ManuallyDrop<MdwHsIn>,
    pub out: std::mem::ManuallyDrop<MdwHsOut>,
    _pad: [u8; 40],
}

ioctl_readwrite!(apu_mdw_handshake, APUSYS_MAGICNO, 32, MdwHsArgs);

// ---- memory (cmd 33) -------------------------------------------

#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum MdwMemOp {
    Alloc = 0,
    Free = 1,
    Map = 2,
    Unmap = 3,
    Flush = 4,
    Invalidate = 5,
}

#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum MdwMemType {
    Main = 0,
    Vlm = 1,
    Local = 2,
    System = 3,
    SystemIsp = 4,
    SystemApu = 5,
}

pub const F_MDW_MEM_CACHEABLE: u64 = 1 << 0;
pub const F_MDW_MEM_32BIT: u64 = 1 << 1;
pub const F_MDW_MEM_HIGHADDR: u64 = 1 << 2;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct MdwMemInAlloc {
    /// MdwMemType selecting the allocator pool.
    pub r#type: u32,
    pub size: u32,
    pub align: u32,
    /// F_MDW_MEM_* — cacheability and DVA-range hints.
    pub flags: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct MdwMemInHandle {
    /// dma-buf-fd-shaped handle returned by Alloc.
    pub handle: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct MdwMemInMap {
    pub handle: u64,
    pub size: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct MdwMemInRange {
    pub handle: u64,
    pub offset: u32,
    pub size: u32,
}

#[repr(C)]
pub union MdwMemInPayload {
    pub alloc: MdwMemInAlloc,
    pub free: MdwMemInHandle,
    pub map: MdwMemInMap,
    pub unmap: MdwMemInHandle,
    pub flush: MdwMemInRange,
    pub invalidate: MdwMemInRange,
    _pad: [u8; 24],
}

#[repr(C)]
pub struct MdwMemIn {
    /// MdwMemOp.
    pub op: u32,
    /// Top-level flags; per-op flags live inside the alloc variant.
    pub flags: u64,
    pub payload: MdwMemInPayload,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct MdwMemOutAlloc {
    pub handle: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct MdwMemOutMap {
    pub r#type: u32,
    pub device_va: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct MdwMemOutImport {
    pub device_va: u64,
    pub size: u32,
}

#[repr(C)]
pub union MdwMemOut {
    pub alloc: MdwMemOutAlloc,
    pub map: MdwMemOutMap,
    pub import: MdwMemOutImport,
}

#[repr(C)]
pub union MdwMemArgs {
    pub r#in: std::mem::ManuallyDrop<MdwMemIn>,
    pub out: std::mem::ManuallyDrop<MdwMemOut>,
    _pad: [u8; 40],
}

ioctl_readwrite!(apu_mdw_mem, APUSYS_MAGICNO, 33, MdwMemArgs);

// ---- command submission (cmd 34) -------------------------------

#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum MdwCmdOp {
    Run = 0,
    RunStale = 1,
    Del = 2,
}

pub const MDW_CB_BIDIRECTIONAL: u32 = 0;
pub const MDW_CB_IN: u32 = 1;
pub const MDW_CB_OUT: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct MdwSubcmdExecInfo {
    pub driver_time: u32,
    pub ip_time: u32,
    pub ip_start_ts: u32,
    pub ip_end_ts: u32,
    pub bw: u32,
    pub boost: u32,
    pub tcm_usage: u32,
    pub ret: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct MdwCmdExecInfo {
    pub sc_rets: u64,
    pub ret: i64,
    pub total_us: u64,
    pub reserved: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct MdwSubcmdCmdbuf {
    /// Handle returned by APU_MDW_IOCTL_MEM(Alloc).
    pub handle: u64,
    pub size: u32,
    pub align: u32,
    /// MDW_CB_BIDIRECTIONAL / MDW_CB_IN / MDW_CB_OUT.
    pub direction: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct MdwSubcmdInfo {
    /// MDW_DEV_* device class for this subcmd.
    pub r#type: u32,
    pub suggest_time: u32,
    pub vlm_usage: u32,
    pub vlm_ctx_id: u32,
    pub vlm_force: u32,
    pub boost: u32,
    pub turbo_boost: u32,
    pub min_boost: u32,
    pub max_boost: u32,
    pub hse_en: u32,
    pub pack_id: u32,
    pub driver_time: u32,
    pub ip_time: u32,
    pub bw: u32,
    pub affinity: u32,
    pub num_cmdbufs: u32,
    /// User pointer to MdwSubcmdCmdbuf[num_cmdbufs].
    pub cmdbufs: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct MdwSubcmdLinkV1 {
    pub producer_idx: u32,
    pub consumer_idx: u32,
    pub vid: u32,
    pub va: u64,
    pub x: u64,
    pub y: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct MdwCmdInExec {
    pub uid: u64,
    pub priority: u32,
    pub hardlimit: u32,
    pub softlimit: u32,
    pub fastmem_ms: u32,
    pub power_save: u32,
    pub power_plcy: u32,
    pub power_dtime: u32,
    pub app_type: u32,
    pub flags: u32,
    pub num_subcmds: u32,
    /// User pointer to MdwSubcmdInfo[num_subcmds].
    pub subcmd_infos: u64,
    /// User pointer to adjacency-matrix bytes.
    pub adj_matrix: u64,
    /// Inbound dma-fence fd to wait on; out-fence written via MdwCmdOutExec.
    pub fence: u64,
    /// User pointer to MdwExecInfo (cmd + per-subcmd) array.
    pub exec_infos: u64,
    pub num_links: u32,
    /// User pointer to MdwSubcmdLinkV1[num_links].
    pub links: u64,
}

#[repr(C)]
pub struct MdwCmdIn {
    /// MdwCmdOp.
    pub op: u32,
    pub reserved: u64,
    /// On Del/RunStale this is the kernel-assigned cmd id.
    pub id: i64,
    pub exec: MdwCmdInExec,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct MdwCmdOutExec {
    pub id: u64,
    /// dma-fence fd userspace can poll/wait on.
    pub fence: u64,
}

#[repr(C)]
pub union MdwCmdOut {
    pub exec: MdwCmdOutExec,
}

#[repr(C)]
pub union MdwCmdArgs {
    pub r#in: std::mem::ManuallyDrop<MdwCmdIn>,
    pub out: std::mem::ManuallyDrop<MdwCmdOut>,
    _pad: [u8; 120],
}

ioctl_readwrite!(apu_mdw_cmd, APUSYS_MAGICNO, 34, MdwCmdArgs);

// ---- util / power / ucmd (cmd 35) ------------------------------

#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub enum MdwUtilOp {
    SetPower = 0,
    Ucmd = 1,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct MdwUtilInPower {
    pub dev_type: u32,
    pub core_idx: u32,
    /// 0..=MDW_BOOST_MAX (100). Driver clamps.
    pub boost: u32,
    pub reserve: u64,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct MdwUtilInUcmd {
    pub dev_type: u32,
    pub size: u32,
    /// Handle of an alloc'd cmdbuf carrying the opaque ucmd payload.
    pub handle: u64,
}

#[repr(C)]
pub union MdwUtilInPayload {
    pub power: MdwUtilInPower,
    pub ucmd: MdwUtilInUcmd,
    _pad: [u8; 24],
}

#[repr(C)]
pub struct MdwUtilIn {
    /// MdwUtilOp.
    pub op: u32,
    pub payload: MdwUtilInPayload,
}

#[repr(C)]
pub union MdwUtilArgs {
    pub r#in: std::mem::ManuallyDrop<MdwUtilIn>,
    _pad: [u8; 32],
}

ioctl_readwrite!(apu_mdw_util, APUSYS_MAGICNO, 35, MdwUtilArgs);

// MDW_BOOST_MAX, MDW_DEFAULT_TIMEOUT_MS etc. live in mdw.h (kernel
// internal) but are useful constants when populating MdwSubcmdInfo.
pub const MDW_BOOST_MAX: u32 = 100;
pub const MDW_DEFAULT_ALIGN: u32 = 16;
pub const MDW_DEFAULT_TIMEOUT_MS: u32 = 30 * 1000;

// Compile-time drift guards. nix's `ioctl_readwrite!` encodes _IOC_SIZE
// from the Rust struct layout. If a future edit changes a struct, the
// encoded ioctl number silently shifts and the kernel rejects with -EINVAL.
// Sizes are gcc sizeof() on the original C header (Genio android13 ==
// NothingOSS Pacman, byte-identical).
use static_assertions::const_assert_eq;
const_assert_eq!(std::mem::size_of::<MdwHsArgs>(), 40);
const_assert_eq!(std::mem::size_of::<MdwMemArgs>(), 40);
const_assert_eq!(std::mem::size_of::<MdwCmdArgs>(), 120);
const_assert_eq!(std::mem::size_of::<MdwUtilArgs>(), 32);
