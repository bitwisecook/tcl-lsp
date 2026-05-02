"""Combine C tclsh + WASM sweep results into a single CSV.

Reads NDJSON from both sweeps and emits a CSV with one row per test
file, side-by-side native + WASM stats.  Output is suitable as a
human-readable baseline doc.
"""

from __future__ import annotations

import csv
import json
import sys
from pathlib import Path


def load(path: Path) -> dict[str, dict]:
    out: dict[str, dict] = {}
    if not path.exists():
        return out
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            rec = json.loads(line)
        except json.JSONDecodeError:
            continue
        if "name" in rec:
            out[rec["name"]] = rec
    return out


def main() -> int:
    import argparse

    p = argparse.ArgumentParser()
    p.add_argument("--c", default="/tmp/c-tcl-sweep.ndjson")
    p.add_argument("--wasm", default="/tmp/tcltest-sweep-fast.ndjson")
    p.add_argument("--out", default="/home/user/tcl-lsp/docs/c-tcl-9.0.3-tcltest-baseline.csv")
    args = p.parse_args()

    c = load(Path(args.c))
    w = load(Path(args.wasm))
    names = sorted(set(c) | set(w))

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)

    def categorise_trap(trap: str) -> str:
        if not trap:
            return ""
        import re as _re

        if _re.search(r"unknown command: [0-9a-f]{8,}", trap):
            return "braced-cmdsubst-leak"  # description like {[abc123]} eval'd
        if "unknown command: Bug" in trap:
            return "braced-cmdsubst-leak"  # description like {[Bug 1234]}
        if "unknown command: " in trap:
            return "unknown-command"
        if "unsupported command:" in trap or "unsupported in WASM" in trap:
            return "unsupported-command"
        if 'wrong # args: should be "test name desc' in trap:
            return "tcltest-arg-parse-fallthrough"
        if "bad option" in trap and "must be -body" in trap:
            return "tcltest-option-parse"
        if "no such variable" in trap:
            return "var-not-set"
        if "rename" in trap and "command doesn't exist" in trap:
            return "rename-missing-cmd"
        if "negative shift" in trap:
            return "expr-negshift"
        if "preserveCore" in trap:
            return "tcltest-preservecore"
        if "ConstraintInitializer" in trap:
            return "info-complete"
        return "other"

    # Comparison policy: WASM compile time is a one-time cost paid by
    # the build harness, not by the running interpreter, so we exclude
    # it from the runtime-vs-runtime comparison.  ``slowdown_x`` is
    # ``wasm_run_ms / c_wall_ms`` — both measure "interpreter ran the
    # script end-to-end" with the same I/O surface.  The native
    # ``c_wall_ms`` includes a fixed ~25 ms tclsh startup; the WASM
    # ``wasm_run_ms`` is the in-process wasmtime call only (no
    # subprocess setup, no compile).  The fused ``wasm_wall_ms``
    # column is kept for readers who want to see total bundle wall
    # cost including subprocess overhead, but it is NOT what the
    # slowdown column compares.
    with out_path.open("w", newline="") as f:
        wr = csv.writer(f)
        wr.writerow(
            [
                "name",
                "subsystem",
                "c_outcome",
                "c_total",
                "c_passed",
                "c_skipped",
                "c_failed",
                "c_wall_ms",
                "c_user_s",
                "c_sys_s",
                "c_max_rss_mb",
                "wasm_outcome",
                "wasm_total",
                "wasm_passed",
                "wasm_skipped",
                "wasm_failed",
                "wasm_run_ms",
                "wasm_pass_rate",
                "slowdown_x",
                "wasm_compile_ms",
                "wasm_wall_ms",
                "wasm_trap_category",
                "wasm_first_failing",
                "wasm_trap",
            ]
        )
        # Read subsystem labels from _IN_SCOPE
        try:
            sys.path.insert(0, "/home/user/tcl-lsp")
            from tests.external.run_tcl9_tests import _IN_SCOPE

            sub_map = dict(_IN_SCOPE)
        except Exception:
            sub_map = {}

        for name in names:
            cr = c.get(name, {})
            wr_ = w.get(name, {})
            c_wall = cr.get("wall_ms")
            w_wall = wr_.get("wall_ms")
            w_run = wr_.get("run_ms")
            slowdown = ""
            # Slowdown is run-only WASM vs C tclsh wall — compile is
            # excluded so the column reflects interpreter cost only.
            if c_wall and w_run and c_wall > 0:
                slowdown = f"{w_run / c_wall:.1f}"
            c_total = cr.get("total", 0) or 0
            w_passed = wr_.get("passed", 0) or 0
            pass_rate = ""
            if c_total > 0:
                pass_rate = f"{100 * w_passed / c_total:.0f}%"
            wr.writerow(
                [
                    name,
                    sub_map.get(name, ""),
                    cr.get("outcome", ""),
                    cr.get("total", ""),
                    cr.get("passed", ""),
                    cr.get("skipped", ""),
                    cr.get("failed", ""),
                    f"{c_wall:.0f}" if c_wall else "",
                    cr.get("user_s", ""),
                    cr.get("sys_s", ""),
                    f"{cr.get('max_rss_kb', 0) / 1024:.1f}" if cr.get("max_rss_kb") else "",
                    wr_.get("outcome", ""),
                    wr_.get("total", ""),
                    wr_.get("passed", ""),
                    wr_.get("skipped", ""),
                    wr_.get("failed", ""),
                    f"{w_run:.0f}" if w_run else "",
                    pass_rate,
                    slowdown,
                    f"{wr_.get('compile_ms', 0):.0f}" if wr_.get("compile_ms") else "",
                    f"{w_wall:.0f}" if w_wall else "",
                    categorise_trap(wr_.get("trap") or ""),
                    (wr_.get("first_failing") or "")[:60],
                    (wr_.get("trap") or "").replace("\n", " | ")[:200],
                ]
            )

    print(f"Wrote {out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
