#!/usr/bin/env python3
"""Crunch tcltest_results.json into the per-file and per-subsystem
tables we need for the markdown sub-reports."""

from __future__ import annotations

import json
import re
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
OUTPUT = REPO / "tmp" / "perf-output"
ROWS = json.loads((OUTPUT / "tcltest_results.json").read_text())


def _classify(row: dict) -> str:
    """Return one of: pass | partial | run-trap | compile-fail | timeout."""
    w = row.get("wasm", {})
    if w.get("compile_error"):
        return "compile-fail"
    if w.get("run_error"):
        return "run-trap"
    if "total" not in w:
        return "no-summary"
    failed = w.get("failed", 0)
    passed = w.get("passed", 0)
    total = w.get("total", 0)
    if total == 0:
        return "no-summary"
    if failed == 0 and passed == total - w.get("skipped", 0):
        return "pass"
    return "partial"


def _ratio(num: float | None, den: float | None) -> float | None:
    if not num or not den:
        return None
    return num / den


def per_file_table():
    print(
        "| stem | sub | bundle KB | wasm KB | compile (ms) | wasm run (ms) | tcl run (ms) | wasm: P/F/S of T | tcl: P/F/S of T | status |"
    )
    print("|---|---|---:|---:|---:|---:|---:|---|---|---|")
    for row in ROWS:
        stem = row["stem"]
        sub = row["subsystem"]
        bundle_kb = row.get("bundle_bytes", 0) / 1024
        w = row.get("wasm", {})
        t = row.get("tclsh", {})
        wasm_kb = w.get("wasm_bytes", 0) / 1024
        c_ms = w.get("compile_ns", 0) / 1e6
        wr_ms = w.get("run_ns", 0) / 1e6
        tr_ms = t.get("run_ns", 0) / 1e6
        if "total" in w:
            wasm_summary = f"{w.get('passed', 0)}/{w.get('failed', 0)}/{w.get('skipped', 0)} of {w.get('total', 0)}"
        elif w.get("compile_error"):
            wasm_summary = "COMPILE-FAIL"
        elif w.get("run_error"):
            wasm_summary = "TRAP"
        else:
            wasm_summary = "no-summary"
        if "total" in t:
            tcl_summary = f"{t.get('passed', 0)}/{t.get('failed', 0)}/{t.get('skipped', 0)} of {t.get('total', 0)}"
        elif t.get("timed_out"):
            tcl_summary = "TIMEOUT"
        else:
            tcl_summary = "no-summary"
        status = _classify(row)
        print(
            f"| {stem} | {sub} | {bundle_kb:.0f} | {wasm_kb:.0f} | {c_ms:.0f} | {wr_ms:.0f} | {tr_ms:.0f} | {wasm_summary} | {tcl_summary} | {status} |"
        )


def per_subsystem_table():
    by = defaultdict(
        lambda: {
            "files": 0,
            "wasm_pass_files": 0,
            "wasm_partial_files": 0,
            "wasm_trap_files": 0,
            "wasm_compile_fail_files": 0,
            "wasm_total_passed": 0,
            "wasm_total_failed": 0,
            "wasm_total_skipped": 0,
            "wasm_total_total": 0,
            "tcl_total_passed": 0,
            "tcl_total_failed": 0,
            "tcl_total_skipped": 0,
            "tcl_total_total": 0,
            "wasm_run_ms": 0.0,
            "wasm_compile_ms": 0.0,
            "tcl_run_ms": 0.0,
            "wasm_bytes": 0,
            "bundle_bytes": 0,
        }
    )
    for row in ROWS:
        sub = row["subsystem"]
        b = by[sub]
        b["files"] += 1
        cls = _classify(row)
        if cls == "pass":
            b["wasm_pass_files"] += 1
        elif cls == "partial":
            b["wasm_partial_files"] += 1
        elif cls == "run-trap":
            b["wasm_trap_files"] += 1
        elif cls == "compile-fail":
            b["wasm_compile_fail_files"] += 1
        w = row.get("wasm", {})
        t = row.get("tclsh", {})
        for src, prefix in ((w, "wasm"), (t, "tcl")):
            for k in ("passed", "failed", "skipped", "total"):
                b[f"{prefix}_total_{k}"] += src.get(k, 0) or 0
        b["wasm_run_ms"] += (w.get("run_ns", 0) or 0) / 1e6
        b["wasm_compile_ms"] += (w.get("compile_ns", 0) or 0) / 1e6
        b["tcl_run_ms"] += (t.get("run_ns", 0) or 0) / 1e6
        b["wasm_bytes"] += w.get("wasm_bytes", 0) or 0
        b["bundle_bytes"] += row.get("bundle_bytes", 0) or 0

    print(
        "| subsystem | files | wasm pass | wasm partial | wasm trap | wasm tests pass/total | tcl tests pass/total |"
        " wasm pass% | tcl pass% | wasm run (ms) | tcl run (ms) | wasm/tcl |"
    )
    print("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    for sub in sorted(by.keys()):
        b = by[sub]
        wp = b["wasm_total_passed"]
        wt = b["wasm_total_total"]
        tp = b["tcl_total_passed"]
        tt = b["tcl_total_total"]
        wpct = (wp / tt * 100) if tt else 0
        tpct = (tp / tt * 100) if tt else 0
        ratio = b["wasm_run_ms"] / b["tcl_run_ms"] if b["tcl_run_ms"] else 0
        print(
            f"| {sub} | {b['files']} | {b['wasm_pass_files']} | {b['wasm_partial_files']} | "
            f"{b['wasm_trap_files'] + b['wasm_compile_fail_files']} | {wp}/{wt} | {tp}/{tt} | "
            f"{wpct:.1f}% | {tpct:.1f}% | {b['wasm_run_ms']:.0f} | {b['tcl_run_ms']:.0f} | "
            f"{ratio:.2f}× |"
        )

    # Totals.
    tot = defaultdict(int)
    for b in by.values():
        for k, v in b.items():
            if isinstance(v, (int, float)):
                tot[k] += v
    twpct = (
        (tot["wasm_total_passed"] / tot["tcl_total_total"] * 100) if tot["tcl_total_total"] else 0
    )
    ttpct = (
        (tot["tcl_total_passed"] / tot["tcl_total_total"] * 100) if tot["tcl_total_total"] else 0
    )
    ratio = tot["wasm_run_ms"] / tot["tcl_run_ms"] if tot["tcl_run_ms"] else 0
    print(
        f"| **TOTAL** | {tot['files']} | {tot['wasm_pass_files']} | {tot['wasm_partial_files']} | "
        f"{tot['wasm_trap_files'] + tot['wasm_compile_fail_files']} | "
        f"{tot['wasm_total_passed']}/{tot['wasm_total_total']} | "
        f"{tot['tcl_total_passed']}/{tot['tcl_total_total']} | {twpct:.1f}% | {ttpct:.1f}% | "
        f"{tot['wasm_run_ms']:.0f} | {tot['tcl_run_ms']:.0f} | {ratio:.2f}× |"
    )


def trap_signatures():
    counts = defaultdict(list)
    for row in ROWS:
        w = row.get("wasm", {})
        if not w.get("run_error"):
            continue
        # Extract trap site from stderr_tail.
        stderr = w.get("stderr_tail", "") or ""
        m = re.search(r"unsupported (?:command|in WASM|sub):? ?([^\n]*)", stderr)
        sig = m.group(0) if m else "other-trap"
        # Trim
        sig = sig.split("\n")[0][:80]
        counts[sig].append(row["stem"])
    print("| trap signature | files affected |")
    print("|---|---|")
    for sig, files in sorted(counts.items(), key=lambda kv: -len(kv[1])):
        first = ", ".join(files[:6])
        more = "" if len(files) <= 6 else f" (+{len(files) - 6} more)"
        print(f"| {sig} | **{len(files)}** — {first}{more} |")


def main():
    section = sys.argv[1] if len(sys.argv) > 1 else "subsystem"
    if section == "files":
        per_file_table()
    elif section == "subsystem":
        per_subsystem_table()
    elif section == "traps":
        trap_signatures()
    else:
        sys.exit(f"unknown section: {section}")


if __name__ == "__main__":
    main()
