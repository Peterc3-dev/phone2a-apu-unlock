mod uapi;

use std::fs::OpenOptions;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::os::fd::AsRawFd;

use uapi::*;

const DEV: &str = "/dev/apusys";

fn main() -> std::io::Result<()> {
    let f = OpenOptions::new().read(true).write(true).open(DEV)?;
    let fd = f.as_raw_fd();
    println!("opened {DEV} fd={fd}");

    let basic = hs_basic(fd)?;
    println!(
        "version={:#x} dev_mask={:#x} mem_mask={:#x} flags={:#x} meta_size={}",
        basic.version, basic.dev_bitmask, basic.mem_bitmask, basic.flags, basic.meta_size
    );

    for bit in 0..64u32 {
        if basic.dev_bitmask & (1u64 << bit) == 0 {
            continue;
        }
        match hs_dev(fd, bit) {
            Ok(d) => {
                let meta = ascii_only(&d.meta);
                println!("  dev[{bit}] num={} meta={:?}", d.num, meta);
            }
            Err(e) => println!("  dev[{bit}] err={e}"),
        }
    }

    for bit in 0..64u32 {
        if basic.mem_bitmask & (1u64 << bit) == 0 {
            continue;
        }
        match hs_mem(fd, bit) {
            Ok(m) => println!("  mem[{bit}] start={:#x} size={:#x}", m.start, m.size),
            Err(e) => println!("  mem[{bit}] err={e}"),
        }
    }

    // TODO: alloc cmdbuf via APU_MDW_IOCTL_MEM(MdwMemOp::Alloc) ->
    //   MdwMemInAlloc { type: MdwMemType::Main, size, align: 16, flags: F_MDW_MEM_CACHEABLE }
    //   handle returned in MdwMemOut::alloc.handle. mmap via the fd
    //   (driver exposes the buffer as a dma-buf), use memmap2.
    // TODO: build MdwSubcmdInfo[] referencing those handles, build
    //   MdwSubcmdInfo::cmdbufs as user pointer to MdwSubcmdCmdbuf[].
    // TODO: APU_MDW_IOCTL_CMD(MdwCmdOp::Run) with MdwCmdInExec; read
    //   out_fence fd from MdwCmdOutExec.fence.
    // TODO: poll(out_fence, POLLIN) until ready, then read MdwCmdExecInfo.
    // TODO: APU_MDW_IOCTL_MEM(Invalidate) on output cmdbufs before reading.

    Ok(())
}

fn hs_basic(fd: i32) -> std::io::Result<MdwHsOutBasic> {
    let mut args = MaybeUninit::<MdwHsArgs>::zeroed();
    unsafe {
        let p = args.as_mut_ptr();
        (*p).r#in = ManuallyDrop::new(MdwHsIn {
            op: MdwHsOp::Basic as u32,
            flags: 0,
            payload: MdwHsInPayload { dev: MdwHsInDev { r#type: 0 } },
        });
        let mut a = args.assume_init();
        apu_mdw_handshake(fd, &mut a).map_err(io_err)?;
        Ok(a.out.basic)
    }
}

fn hs_dev(fd: i32, ty: u32) -> std::io::Result<MdwHsOutDev> {
    let mut args = MaybeUninit::<MdwHsArgs>::zeroed();
    unsafe {
        let p = args.as_mut_ptr();
        (*p).r#in = ManuallyDrop::new(MdwHsIn {
            op: MdwHsOp::Dev as u32,
            flags: 0,
            payload: MdwHsInPayload { dev: MdwHsInDev { r#type: ty } },
        });
        let mut a = args.assume_init();
        apu_mdw_handshake(fd, &mut a).map_err(io_err)?;
        Ok(a.out.dev)
    }
}

fn hs_mem(fd: i32, ty: u32) -> std::io::Result<MdwHsOutMem> {
    let mut args = MaybeUninit::<MdwHsArgs>::zeroed();
    unsafe {
        let p = args.as_mut_ptr();
        (*p).r#in = ManuallyDrop::new(MdwHsIn {
            op: MdwHsOp::Mem as u32,
            flags: 0,
            payload: MdwHsInPayload { mem: MdwHsInMem { r#type: ty } },
        });
        let mut a = args.assume_init();
        apu_mdw_handshake(fd, &mut a).map_err(io_err)?;
        Ok(a.out.mem)
    }
}

fn ascii_only(b: &[u8]) -> String {
    let n = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..n]).into_owned()
}

fn io_err(e: nix::errno::Errno) -> std::io::Error {
    std::io::Error::from_raw_os_error(e as i32)
}
