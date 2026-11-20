#!/usr/bin/env python3
"""Extract TMM's Tcl command registration table and function starts.

TMM (tmm64.pgo_use) registers every iRule command in a dedicated ELF section,
`set_tclcmd`: a 16-byte header, then 32-byte records
{name pointer, handler, clientdata, 0}. Many commands share a handler and
are told apart by clientdata. Ghidra does not load this section's contents,
so read it from the file.

Usage: extract-tmm-cmdtable.py <tmm64.pgo_use> <table.tsv> [<fde-starts.txt>]
  table.tsv       name<TAB>handler<TAB>clientdata<TAB>0
  fde-starts.txt  every function start from .eh_frame_hdr (for creating the
                  functions Ghidra misses; see ghidra/*.java)
"""
import struct
import sys


def main(path, out, fde_out=None):
    data = open(path, "rb").read()
    shoff = struct.unpack_from("<Q", data, 0x28)[0]
    shentsize, shnum, shstrndx = struct.unpack_from("<HHH", data, 0x3A)
    secs = []
    for i in range(shnum):
        name, _t, _f, addr, off, size = struct.unpack_from("<IIQQQQ", data, shoff + i * shentsize)
        secs.append([name, addr, off, size])
    stro = secs[shstrndx][2]
    for s in secs:
        s[0] = data[stro + s[0]: data.index(b"\0", stro + s[0])].decode()

    def va2off(va):
        for n, a, o, sz in secs:
            if a and a <= va < a + sz and n != ".bss":
                return va - a + o

    def cstr(va):
        o = va2off(va)
        if o is None:
            return None
        e = data.find(b"\0", o, o + 120)
        s = data[o:e] if e > 0 else b""
        return s.decode() if s and all(32 <= b < 127 for b in s) else None

    _, a, o, sz = next(s for s in secs if s[0] == "set_tclcmd")
    rows = []
    for va in range(a + 16, a + sz - 31, 32):
        p, fn, cd, z = struct.unpack_from("<QQQQ", data, va2off(va))
        name = cstr(p)
        if name:
            rows.append(f"{name}\t{fn:#x}\t{cd:#x}\t{z:#x}")
    open(out, "w").write("\n".join(rows) + "\n")
    print(len(rows), "commands")

    if fde_out:
        _, va, off, _ = next(s for s in secs if s[0] == ".eh_frame_hdr")
        ver, _ptr_enc, count_enc, table_enc = data[off:off + 4]
        assert (ver, count_enc, table_enc) == (1, 0x03, 0x3B)
        count = struct.unpack_from("<I", data, off + 8)[0]
        starts = sorted(va + struct.unpack_from("<i", data, off + 12 + 8 * i)[0] for i in range(count))
        open(fde_out, "w").write("\n".join(hex(x) for x in starts) + "\n")
        print(count, "function starts")


if __name__ == "__main__":
    main(*sys.argv[1:4])
