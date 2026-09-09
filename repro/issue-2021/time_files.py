#!/usr/bin/env python3
"""Per-file analysis cost: time_files.py <tcl-bin> <subcommand> <corpus-dir> <out.tsv> [--timeout S]

Runs `tcl <sub> <file>` one file at a time, records wall seconds, peak RSS KiB
(rusage of the child), line count and path. Prints the 30 slowest at the end.
"""

import argparse, os, resource, subprocess, sys, time
from pathlib import Path

EXTS = {".tcl", ".qsf", ".qpf", ".qip", ".sdc"}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("bin")
    ap.add_argument("sub")
    ap.add_argument("corpus")
    ap.add_argument("out")
    ap.add_argument("--timeout", type=float, default=600.0)
    ap.add_argument(
        "--shard", default="0/1", help="i/n: process files with index % n == i"
    )
    a = ap.parse_args()
    files = sorted(
        p for p in Path(a.corpus).rglob("*") if p.is_file() and p.suffix.lower() in EXTS
    )
    si, sn = (int(x) for x in a.shard.split("/"))
    files = [f for k, f in enumerate(files) if k % sn == si]
    rows = []
    with open(a.out, "w") as out:
        for i, f in enumerate(files, 1):
            lines = sum(1 for _ in open(f, "rb"))
            before = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss
            t0 = time.monotonic()
            status = "ok"
            try:
                subprocess.run(
                    [a.bin, a.sub, str(f)],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    timeout=a.timeout,
                )
            except subprocess.TimeoutExpired:
                status = "timeout"
            wall = time.monotonic() - t0
            rss = resource.getrusage(
                resource.RUSAGE_CHILDREN
            ).ru_maxrss  # high-water mark across children
            row = (wall, rss, lines, status, str(f))
            rows.append(row)
            out.write("%.3f\t%d\t%d\t%s\t%s\n" % row)
            out.flush()
            if i % 100 == 0:
                print(
                    f"[{i}/{len(files)}] cumulative {sum(r[0] for r in rows):.0f}s",
                    file=sys.stderr,
                )
    rows.sort(reverse=True)
    print(
        f"files={len(rows)} total_wall={sum(r[0] for r in rows):.1f}s timeouts={sum(r[3] == 'timeout' for r in rows)}"
    )
    for r in rows[:30]:
        print("%.2fs\t%d KiB\t%d lines\t%s\t%s" % r)


if __name__ == "__main__":
    main()
