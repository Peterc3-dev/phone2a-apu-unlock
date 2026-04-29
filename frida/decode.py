#!/usr/bin/env python3
"""
decode.py — joins captured /sdcard/apu_ioctl.jsonl against the apusys ABI
catalogue and prints a labelled, ordered ioctl trace plus a histogram.

Usage:
    adb pull /sdcard/apu_ioctl.jsonl
    ./decode.py apu_ioctl.jsonl ../abi/apusys_ioctl_abi.json
"""
import json
import sys
from collections import Counter
from pathlib import Path


def load_abi(p: Path) -> dict[int, dict]:
    raw = json.loads(p.read_text())
    ioctls = raw["ioctls"] if isinstance(raw, dict) else raw
    return {int(e["cmd_hex"], 16): e for e in ioctls}


def main(trace: Path, abi: Path) -> None:
    catalog = load_abi(abi)
    cmd_counts: Counter[int] = Counter()
    err_counts: Counter[int] = Counter()
    print(f"# {trace.name}: trace decoded against {abi.name}\n")
    for line in trace.read_text().splitlines():
        if not line.strip():
            continue
        rec = json.loads(line)
        cmd = int(rec["cmd"], 16)
        entry = catalog.get(cmd)
        sym = entry["name"] if entry else f"UNKNOWN(type={rec['type']!r}, nr={rec['nr']})"
        ret = rec.get("ret", "?")
        line = f"[{rec['ts']}] pid={rec['proc']} fd={rec['path']} cmd={rec['cmd']} {sym} ret={ret}"
        if rec.get("errno"):
            line += f" errno={rec['errno']}"
            err_counts[cmd] += 1
        cmd_counts[cmd] += 1
        print(line)
    print("\n# histogram")
    for cmd, n in cmd_counts.most_common():
        sym = catalog.get(cmd, {}).get("name", "UNKNOWN")
        errs = err_counts.get(cmd, 0)
        print(f"  {hex(cmd):>12}  {n:>5}  {sym}" + (f"  (errors: {errs})" if errs else ""))


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("usage: decode.py <trace.jsonl> <abi.json>", file=sys.stderr)
        sys.exit(2)
    main(Path(sys.argv[1]), Path(sys.argv[2]))
