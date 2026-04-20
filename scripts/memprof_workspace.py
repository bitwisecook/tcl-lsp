"""Measure memory footprint of BackgroundScanner + WorkspaceIndex on a directory.

Usage:
    uv run python scripts/memprof_workspace.py <path> [--limit N] [--cap-gb N]

Reports peak RSS, tracemalloc top allocations, and per-file averages so we can
compare before/after the stripping fix.

``--limit N`` analyses at most N files (useful for quick baselines on huge
trees). ``--cap-gb N`` sets RLIMIT_AS so the process cannot exceed that much
address space — a safety net against accidental OOM.
"""

from __future__ import annotations

import argparse
import gc
import os
import resource
import sys
import time
import tracemalloc
from pathlib import Path


def rss_kb() -> int:
    return resource.getrusage(resource.RUSAGE_SELF).ru_maxrss


def rss_mb() -> float:
    return rss_kb() / 1024.0


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("target")
    ap.add_argument("--limit", type=int, default=0, help="analyse at most N files (0 = all)")
    ap.add_argument(
        "--cap-gb", type=float, default=6.0, help="address space cap in GB (0 = no cap)"
    )
    args = ap.parse_args()

    if args.cap_gb > 0:
        cap = int(args.cap_gb * 1024 * 1024 * 1024)
        resource.setrlimit(resource.RLIMIT_AS, (cap, cap))

    target = os.path.abspath(os.path.expanduser(args.target))
    if not os.path.isdir(target):
        print(f"Not a directory: {target}")
        sys.exit(1)

    # Import after argv parsing so import-time cost is measured too.
    from lsp.workspace.scanner import BackgroundScanner
    from lsp.workspace.workspace_index import EntrySource, WorkspaceIndex

    gc.collect()
    tracemalloc.start(25)

    t0 = time.perf_counter()
    rss_before = rss_mb()
    snap_before = tracemalloc.take_snapshot()

    scanner = BackgroundScanner()
    scanner.configure(workspace_roots=[target])

    files = scanner.collect_files()
    if args.limit:
        files = files[: args.limit]
    total = len(files)
    print(f"scanning {total} files from {target}", flush=True)

    t_last = time.perf_counter()
    for i, (full_path, ext) in enumerate(files, start=1):
        scanner.analyse_one(full_path, ext)
        now = time.perf_counter()
        if now - t_last > 5.0 or i == total:
            print(f"  [{i}/{total}] RSS={rss_mb():.0f}MB  elapsed={now - t0:.0f}s", flush=True)
            t_last = now

    index = WorkspaceIndex()
    for uri, scan_result in scanner._cached.items():
        index.update(uri, scan_result.analysis, EntrySource.BACKGROUND)
    for uri, procs in scanner.irules_procs.items():
        index.update_irules_globals(uri, procs)
    for uri, exports in scanner.irules_rule_init_vars.items():
        index.update_rule_init_vars(uri, exports)

    elapsed = time.perf_counter() - t0
    gc.collect()
    rss_after = rss_mb()
    snap_after = tracemalloc.take_snapshot()

    n_files = len(scanner._cached)
    print(f"\ntarget:  {target}")
    print(f"files:   {n_files}")
    print(f"time:    {elapsed:.1f}s")
    print(
        f"RSS:     before={rss_before:.0f}MB  after={rss_after:.0f}MB  delta={rss_after - rss_before:.0f}MB"
    )
    if n_files:
        print(f"avg:     {(rss_after - rss_before) * 1024 / n_files:.1f}KB / file")

    stats = snap_after.compare_to(snap_before, "lineno")
    print("\nTop 15 growth sites (tracemalloc):")
    for stat in stats[:15]:
        frame = stat.traceback[0]
        path = Path(frame.filename).name
        kb = stat.size_diff / 1024
        count = stat.count_diff
        print(f"  {kb:+9.1f} KB  n={count:+7d}  {path}:{frame.lineno}")

    tracemalloc.stop()


if __name__ == "__main__":
    main()
