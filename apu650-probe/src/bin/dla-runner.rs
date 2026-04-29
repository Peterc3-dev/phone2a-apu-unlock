//! dla-runner — submit a DLA artifact + input tensor to the APU, read result.
//!
//! Usage:
//!     dla-runner <model.dla> <input.bin> <output.bin> [output_size_bytes]
//!
//! Flow (per midware/2.0/mdw_ioctl.h):
//!     1. open /dev/apusys
//!     2. HANDSHAKE/BASIC: confirm version + meta_size
//!     3. MEM(Alloc) for DLA bytecode → mmap → memcpy(.dla)
//!     4. MEM(Alloc) for input tensor → mmap → memcpy(input.bin)
//!     5. MEM(Alloc) for output tensor (zeroed)
//!     6. Build MdwSubcmdInfo referencing those handles via MdwSubcmdCmdbuf[]
//!     7. CMD(Run) → receive fence fd
//!     8. poll(fence_fd, POLLIN) with timeout
//!     9. MEM(Invalidate) on output (CPU view sees device writes)
//!    10. mmap output handle → read → write to output.bin
//!    11. MEM(Free) all three
//!
//! WHAT THIS WILL GET WRONG ON FIRST RUN
//! - The MdwSubcmdInfo.type field — we don't know if MDLA wants 1, 2, 0x55, etc.
//!   Best guess: bit 1 from the handshake's dev_bitmask (the value 0x2 if
//!   single-bit, else iterate). Validate against a Frida trace from neuronrt.
//! - Whether the DLA is one cmdbuf or many. Genio examples submit one DLA per
//!   subcmd with direction=IN. Stick to that.
//! - num_links / adj_matrix for a single subcmd: 0 / null pointer is the safe
//!   default. If the kernel returns -EINVAL, we may need a 1-byte zero matrix.
//! - Input/output tensor layout. The DLA bakes in expected shapes; we just
//!   need to honor sizeof(input) and sizeof(output) as opaque buffers.
//!
//! All the above can be fixed iteratively by capturing real ioctl traffic
//! with the frida script and diffing.

use anyhow::{anyhow, bail, Context, Result};
use apu650_probe::uapi::*;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::mem::{ManuallyDrop, MaybeUninit};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::path::PathBuf;
use std::ptr;

const APU_DEV: &str = "/dev/apusys";
const FENCE_TIMEOUT_MS: i32 = 30_000;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("usage: dla-runner <model.dla> <input.bin> <output.bin> [output_size_bytes]");
        std::process::exit(64);
    }
    let dla_path = PathBuf::from(&args[1]);
    let in_path = PathBuf::from(&args[2]);
    let out_path = PathBuf::from(&args[3]);
    let out_size_hint: Option<usize> = args.get(4).and_then(|s| s.parse().ok());

    let dev = OpenOptions::new()
        .read(true)
        .write(true)
        .open(APU_DEV)
        .with_context(|| format!("open {APU_DEV}"))?;
    let fd = dev.as_raw_fd();

    let basic = handshake_basic(fd)?;
    eprintln!(
        "[dla-runner] APU version=0x{:x} dev_mask=0x{:x} mem_mask=0x{:x} meta_size={}",
        basic.version, basic.dev_bitmask, basic.mem_bitmask, basic.meta_size
    );

    // Pick the device class. dev_mask bit 1 is conventionally MDLA in the
    // captures we have; if the user knows otherwise, env override helps.
    let dev_type: u32 = std::env::var("APU_DEV_TYPE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            (1..=63u32)
                .find(|b| basic.dev_bitmask & (1u64 << b) != 0)
                .unwrap_or(1)
        });
    eprintln!("[dla-runner] dispatching to dev_type={dev_type}");

    let dla_bytes = read_all(&dla_path)?;
    let in_bytes = read_all(&in_path)?;
    let out_size = out_size_hint.unwrap_or(in_bytes.len()); // fallback: same as input

    // 3..5: allocate three dma-buf-backed regions.
    let dla_h = mem_alloc(fd, dla_bytes.len() as u32, MdwMemType::Main as u32, F_MDW_MEM_CACHEABLE)?;
    let in_h = mem_alloc(fd, in_bytes.len() as u32, MdwMemType::Main as u32, F_MDW_MEM_CACHEABLE)?;
    let out_h = mem_alloc(fd, out_size as u32, MdwMemType::Main as u32, F_MDW_MEM_CACHEABLE)?;

    // mmap each handle (handle is a dma-buf fd on this driver) and copy bytes in.
    write_dmabuf(dla_h, &dla_bytes)?;
    write_dmabuf(in_h, &in_bytes)?;
    zero_dmabuf(out_h, out_size)?;
    // CPU→device sync on the buffers we wrote to.
    mem_flush(fd, dla_h, dla_bytes.len() as u32)?;
    mem_flush(fd, in_h, in_bytes.len() as u32)?;

    // 6: build subcmd referencing the three cmdbufs.
    let cmdbufs = [
        MdwSubcmdCmdbuf {
            handle: dla_h,
            size: dla_bytes.len() as u32,
            align: MDW_DEFAULT_ALIGN,
            direction: MDW_CB_IN,
        },
        MdwSubcmdCmdbuf {
            handle: in_h,
            size: in_bytes.len() as u32,
            align: MDW_DEFAULT_ALIGN,
            direction: MDW_CB_IN,
        },
        MdwSubcmdCmdbuf {
            handle: out_h,
            size: out_size as u32,
            align: MDW_DEFAULT_ALIGN,
            direction: MDW_CB_OUT,
        },
    ];

    let subcmd = MdwSubcmdInfo {
        r#type: dev_type,
        suggest_time: 0,
        vlm_usage: 0,
        vlm_ctx_id: 0,
        vlm_force: 0,
        boost: MDW_BOOST_MAX,
        turbo_boost: 0,
        min_boost: 0,
        max_boost: MDW_BOOST_MAX,
        hse_en: 0,
        pack_id: 0,
        driver_time: 0,
        ip_time: 0,
        bw: 0,
        affinity: 0,
        num_cmdbufs: cmdbufs.len() as u32,
        cmdbufs: cmdbufs.as_ptr() as u64,
    };

    let subcmds = [subcmd];
    let mut exec_info = MdwCmdExecInfo::default();
    let exec_infos = [exec_info; 1];

    // 7: submit.
    let fence_fd = cmd_run(fd, &subcmds, &exec_infos, MDW_DEFAULT_TIMEOUT_MS)?;
    eprintln!("[dla-runner] submitted, fence_fd={fence_fd}");

    // 8: wait on the fence fd via poll().
    poll_fence(fence_fd, FENCE_TIMEOUT_MS)?;
    let _close = unsafe { OwnedFd::from_raw_fd(fence_fd) };
    // Pull the per-subcmd return code out of the (now-completed) exec_info.
    exec_info = exec_infos[0];
    if exec_info.ret != 0 {
        bail!(
            "subcmd reported ret={} (sc_rets=0x{:x}, total_us={})",
            exec_info.ret,
            exec_info.sc_rets,
            exec_info.total_us
        );
    }
    eprintln!("[dla-runner] complete, total_us={}", exec_info.total_us);

    // 9: invalidate output cache so CPU sees device writes.
    mem_invalidate(fd, out_h, out_size as u32)?;
    let out_bytes = read_dmabuf(out_h, out_size)?;

    // 10: write output.
    File::create(&out_path)
        .with_context(|| format!("create {}", out_path.display()))?
        .write_all(&out_bytes)?;
    eprintln!("[dla-runner] wrote {} bytes to {}", out_bytes.len(), out_path.display());

    // 11: free.
    mem_free(fd, dla_h)?;
    mem_free(fd, in_h)?;
    mem_free(fd, out_h)?;
    Ok(())
}

fn read_all(p: &std::path::Path) -> Result<Vec<u8>> {
    let mut v = Vec::new();
    File::open(p)
        .with_context(|| format!("open {}", p.display()))?
        .read_to_end(&mut v)?;
    Ok(v)
}

fn handshake_basic(fd: RawFd) -> Result<MdwHsOutBasic> {
    let mut args = MaybeUninit::<MdwHsArgs>::zeroed();
    unsafe {
        let p = args.as_mut_ptr();
        (*p).r#in = ManuallyDrop::new(MdwHsIn {
            op: MdwHsOp::Basic as u32,
            flags: 0,
            payload: MdwHsInPayload { dev: MdwHsInDev { r#type: 0 } },
        });
        let mut a = args.assume_init();
        apu_mdw_handshake(fd, &mut a).map_err(io)?;
        Ok(a.out.basic)
    }
}

fn mem_alloc(fd: RawFd, size: u32, mem_type: u32, flags: u64) -> Result<u64> {
    let mut args = MaybeUninit::<MdwMemArgs>::zeroed();
    unsafe {
        let p = args.as_mut_ptr();
        (*p).r#in = ManuallyDrop::new(MdwMemIn {
            op: MdwMemOp::Alloc as u32,
            flags: 0,
            payload: MdwMemInPayload {
                alloc: MdwMemInAlloc { r#type: mem_type, size, align: MDW_DEFAULT_ALIGN, flags },
            },
        });
        let mut a = args.assume_init();
        apu_mdw_mem(fd, &mut a).map_err(io)?;
        Ok(a.out.alloc.handle)
    }
}

fn mem_free(fd: RawFd, handle: u64) -> Result<()> {
    let mut args = MaybeUninit::<MdwMemArgs>::zeroed();
    unsafe {
        let p = args.as_mut_ptr();
        (*p).r#in = ManuallyDrop::new(MdwMemIn {
            op: MdwMemOp::Free as u32,
            flags: 0,
            payload: MdwMemInPayload { free: MdwMemInHandle { handle } },
        });
        let mut a = args.assume_init();
        apu_mdw_mem(fd, &mut a).map_err(io)?;
    }
    Ok(())
}

fn mem_flush(fd: RawFd, handle: u64, size: u32) -> Result<()> {
    sync(fd, handle, size, MdwMemOp::Flush)
}
fn mem_invalidate(fd: RawFd, handle: u64, size: u32) -> Result<()> {
    sync(fd, handle, size, MdwMemOp::Invalidate)
}
fn sync(fd: RawFd, handle: u64, size: u32, op: MdwMemOp) -> Result<()> {
    let mut args = MaybeUninit::<MdwMemArgs>::zeroed();
    unsafe {
        let p = args.as_mut_ptr();
        (*p).r#in = ManuallyDrop::new(MdwMemIn {
            op: op as u32,
            flags: 0,
            payload: MdwMemInPayload {
                flush: MdwMemInRange { handle, offset: 0, size },
            },
        });
        let mut a = args.assume_init();
        apu_mdw_mem(fd, &mut a).map_err(io)?;
    }
    Ok(())
}

fn cmd_run(
    fd: RawFd,
    subcmds: &[MdwSubcmdInfo],
    exec_infos: &[MdwCmdExecInfo],
    timeout_ms: u32,
) -> Result<RawFd> {
    let mut args = MaybeUninit::<MdwCmdArgs>::zeroed();
    unsafe {
        let p = args.as_mut_ptr();
        (*p).r#in = ManuallyDrop::new(MdwCmdIn {
            op: MdwCmdOp::Run as u32,
            reserved: 0,
            id: 0,
            exec: MdwCmdInExec {
                uid: 0,
                priority: 0,
                hardlimit: timeout_ms,
                softlimit: 0,
                fastmem_ms: 0,
                power_save: 0,
                power_plcy: 0,
                power_dtime: 0,
                app_type: 0,
                flags: 0,
                num_subcmds: subcmds.len() as u32,
                subcmd_infos: subcmds.as_ptr() as u64,
                adj_matrix: 0, // null; single-subcmd graphs need no adjacency
                fence: 0,      // no inbound fence
                exec_infos: exec_infos.as_ptr() as u64,
                num_links: 0,
                links: 0,
            },
        });
        let mut a = args.assume_init();
        apu_mdw_cmd(fd, &mut a).map_err(io)?;
        Ok(a.out.exec.fence as RawFd)
    }
}

fn poll_fence(fd: RawFd, timeout_ms: i32) -> Result<()> {
    let mut pfd = libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
    let n = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };
    if n < 0 {
        return Err(io(nix::errno::Errno::last()));
    }
    if n == 0 {
        bail!("fence wait timed out after {timeout_ms} ms");
    }
    if pfd.revents & libc::POLLIN == 0 {
        bail!("fence revents=0x{:x}, no POLLIN", pfd.revents);
    }
    Ok(())
}

fn write_dmabuf(handle: u64, data: &[u8]) -> Result<()> {
    let map = mmap_handle(handle, data.len(), libc::PROT_READ | libc::PROT_WRITE)?;
    unsafe {
        ptr::copy_nonoverlapping(data.as_ptr(), map.ptr as *mut u8, data.len());
    }
    munmap(map)
}

fn zero_dmabuf(handle: u64, size: usize) -> Result<()> {
    let map = mmap_handle(handle, size, libc::PROT_READ | libc::PROT_WRITE)?;
    unsafe {
        ptr::write_bytes(map.ptr as *mut u8, 0, size);
    }
    munmap(map)
}

fn read_dmabuf(handle: u64, size: usize) -> Result<Vec<u8>> {
    let map = mmap_handle(handle, size, libc::PROT_READ)?;
    let mut v = vec![0u8; size];
    unsafe {
        ptr::copy_nonoverlapping(map.ptr as *const u8, v.as_mut_ptr(), size);
    }
    munmap(map)?;
    Ok(v)
}

struct Map {
    ptr: *mut libc::c_void,
    size: usize,
}

fn mmap_handle(handle: u64, size: usize, prot: i32) -> Result<Map> {
    let fd = handle as RawFd;
    let ptr = unsafe { libc::mmap(ptr::null_mut(), size, prot, libc::MAP_SHARED, fd, 0) };
    if ptr == libc::MAP_FAILED {
        return Err(anyhow!(
            "mmap(handle=0x{handle:x}, size={size}) failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(Map { ptr, size })
}

fn munmap(m: Map) -> Result<()> {
    if unsafe { libc::munmap(m.ptr, m.size) } < 0 {
        return Err(anyhow!("munmap: {}", std::io::Error::last_os_error()));
    }
    Ok(())
}

fn io(e: nix::errno::Errno) -> anyhow::Error {
    anyhow!(std::io::Error::from_raw_os_error(e as i32))
}
