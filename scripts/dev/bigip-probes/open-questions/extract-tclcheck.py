#!/usr/bin/env python3
"""Extract the BIG-IP rule checker's Tcl sources from libtclCheck_tcl.so.

mcpd validates every iRule at load with a Tcl program derived from the Ajuba
(TclPro) checker. Its scripts are embedded in libtclCheck_tcl.so as exported
read-only symbols named `*_tcl` (checker_tcl, jet_tcl, f5Cmds_tcl,
jet_event_tcl, ...), each NUL-terminated and XORed with a single byte, 0xF5.

Usage: extract-tclcheck.py <libtclCheck_tcl.so.1.0 (64-bit)> <outdir>

Keep the extracted vendor source in private storage outside this repository.
Requires `nm` (binutils or llvm) on PATH.
"""
import pathlib
import struct
import subprocess
import sys

KEY = 0xF5


def sections(data):
    shoff = struct.unpack_from("<Q", data, 0x28)[0]
    shentsize, shnum = struct.unpack_from("<HH", data, 0x3A)
    for i in range(shnum):
        _name, _typ, _flags, addr, off, size = struct.unpack_from("<IIQQQQ", data, shoff + i * shentsize)
        yield addr, off, size


def main(lib, outdir):
    data = pathlib.Path(lib).read_bytes()
    if data[:5] != b"\x7fELF\x02":
        sys.exit("expected a 64-bit ELF")
    secs = list(sections(data))

    def file_offset(va):
        for addr, off, size in secs:
            if addr <= va < addr + size:
                return va - addr + off
        raise ValueError(hex(va))

    nm = subprocess.run(["nm", "-D", "--defined-only", lib], capture_output=True, text=True, check=True)
    syms = sorted((int(a, 16), n) for a, _t, n in (l.split() for l in nm.stdout.splitlines()) if n.endswith("_tcl"))
    out = pathlib.Path(outdir)
    out.mkdir(parents=True, exist_ok=True)
    for va, name in syms:
        start = file_offset(va)
        end = data.index(b"\0", start)
        text = bytes(b ^ KEY for b in data[start:end])
        (out / f"{name}.tcl").write_bytes(text)
        print(f"{name:34s} {text.count(b'\n'):6d} lines")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
