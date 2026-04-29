// apu_ioctl_tap.js — captures every ioctl on /dev/apu* nodes.
//
// Usage:
//   frida -U -f android.hardware.neuralnetworks@1.3-service.mediatek -l apu_ioctl_tap.js
//   frida -U -n com.nothing.camera -l apu_ioctl_tap.js
//
// Output appended to /sdcard/apu_ioctl.jsonl (one JSON record per ioctl).

const log = new File("/sdcard/apu_ioctl.jsonl", "a");
const ioctl = Module.getExportByName("libc.so", "ioctl");
const readlink = new NativeFunction(
    Module.getExportByName("libc.so", "readlink"),
    "int",
    ["pointer", "pointer", "int"]
);

// PATH_MAX on Android is 4096; symlinks for namespaced devices easily exceed 256.
const PATH_BUF_LEN = 4096;
function fdPath(fd) {
    const p = Memory.allocUtf8String("/proc/self/fd/" + fd);
    const buf = Memory.alloc(PATH_BUF_LEN);
    const n = readlink(p, buf, PATH_BUF_LEN - 1);
    return n > 0 ? buf.readUtf8String(n) : "";
}

function decodeCmd(c) {
    return {
        dir:  (c >>> 30) & 0x3,
        size: (c >>> 16) & 0x3fff,
        type: String.fromCharCode((c >>> 8) & 0xff),
        nr:    c         & 0xff,
        raw:  "0x" + (c >>> 0).toString(16),
    };
}

Interceptor.attach(ioctl, {
    onEnter(args) {
        const fd = args[0].toInt32();
        const path = fdPath(fd);
        if (!/^\/dev\/(apu|mdla|vpu|mtk_aov|edma|mdw)/.test(path)) {
            this.skip = true;
            return;
        }
        this.skip = false;
        this.fd = fd;
        this.path = path;
        this.cmd = args[1].toInt32() >>> 0;
        this.arg = args[2];
        const decoded = decodeCmd(this.cmd);
        // Some ioctls advertise size 0 in _IOC_SIZE but still pass a meaningful pointer.
        // Always dump at least 32 bytes to avoid blind spots; cap at 256.
        const dump_len = Math.min(Math.max(decoded.size, 32), 256);
        let dump = "";
        try {
            dump = hexdump(this.arg, { length: dump_len, header: false, ansi: false });
        } catch (e) {
            dump = "<unreadable: " + e.message + ">";
        }
        const rec = {
            ts:   Date.now(),
            tid:  Process.getCurrentThreadId(),
            proc: Process.id,
            path: this.path,
            cmd:  decoded.raw,
            dir:  decoded.dir,
            size: decoded.size,
            type: decoded.type,
            nr:   decoded.nr,
            arg:  this.arg.toString(),
            payload_hex: dump,
        };
        this.rec = rec;
    },
    onLeave(retval) {
        if (this.skip) return;
        this.rec.ret = retval.toInt32();
        if (this.rec.ret < 0) {
            try {
                this.rec.errno = -this.rec.ret;
            } catch (e) {}
        }
        log.write(JSON.stringify(this.rec) + "\n");
        log.flush();
    },
});

console.log("[apu_ioctl_tap] armed");
