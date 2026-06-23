#!/usr/bin/env python3
"""Gap map: Rust bytecode VM (`rust/tcl-vm`) vs C Tcl 9 across the Tier 1/2/3
foundational tcltest suites.

Runs each `tmp/tcl9.0.3/tests/<stem>.test` through the `tcl-vm` `run_test`
example (the bytecode VM sourcing the real `tcltest.tcl`), parses tcltest's
summary line, and compares the (passed, skipped, failed) triple against the C
reference in `tests/baselines/tcl9-tcltest/c-tclsh.ndjson`.

Goal parity = the VM's (passed, skipped, failed) EXACTLY matches C for a stem.

Usage:
    TCL_LIBRARY=tmp/tcl9.0.3/library python3 scripts/dev/rust_vm_tier_gap.py \
        [--tier 1|2|3|all] [--timeout S] [--json out.json] [stems...]
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
TESTS = REPO / "tmp" / "tcl9.0.3" / "tests"
RUN_TEST = REPO / "target" / "release" / "examples" / "run_test"
C_REF = REPO / "tests" / "baselines" / "tcl9-tcltest" / "c-tclsh.ndjson"

TIERS: dict[str, list[str]] = {
    "1": [
        # parser / lexer / word-level
        "parse", "parseOld", "parseExpr", "word", "subst",
        # bytecode compiler & VM internals
        "compile", "execute", "basic", "misc", "compExpr", "compExpr-old",
        "appendComp", "lsetComp", "regexpComp", "assemble", "obj", "nre",
        # eval / control flow / errors
        "eval", "if", "if-old", "for", "for-old", "foreach", "while",
        "while-old", "switch", "error", "result",
        # expressions & math
        "expr", "expr-old", "mathop", "incr", "incr-old",
        # procs / scope / namespaces
        "proc", "proc-old", "apply", "uplevel", "upvar", "namespace",
        "namespace-old", "var",
        # introspection / runtime services
        "info", "cmdInfo", "trace", "rename", "unknown",
        # non-I/O data commands — strings
        "string", "set", "set-old", "append", "format", "scan", "split",
        "join", "concat", "regexp", "stringObj", "dstring",
        # lists
        "list", "lindex", "linsert", "llength", "lrange", "lrepeat",
        "lreplace", "lsearch", "lset", "lmap", "lpop", "lseq", "listObj",
        "listRep", "abstractlist",
        # grouped command suites + helpers
        "cmdAH", "cmdIL", "cmdMZ", "util", "dict", "indexObj", "get", "assocd",
    ],
    "2": [
        "coroutine", "tailcall", "interp", "binary",
        "oo", "ooNext2", "ooProp", "ooUtil",
    ],
    "3": [
        "encoding", "utf", "clock", "msgcat",
        "safe", "safe-stock", "safe-stock86", "safe-zipfs",
    ],
}

SUMMARY_RE = re.compile(
    r"Total\s+(\d+)\s+Passed\s+(\d+)\s+Skipped\s+(\d+)\s+Failed\s+(\d+)"
)


def load_c_ref() -> dict[str, tuple[int, int, int, int]]:
    ref: dict[str, tuple[int, int, int, int]] = {}
    if not C_REF.exists():
        return ref
    for line in C_REF.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        d = json.loads(line)
        ref[d["name"]] = (
            int(d.get("total", 0)),
            int(d.get("passed", 0)),
            int(d.get("skipped", 0)),
            int(d.get("failed", 0)),
        )
    return ref


def run_vm(stem: str, timeout: int) -> dict:
    test = TESTS / f"{stem}.test"
    if not test.exists():
        return {"outcome": "absent"}
    try:
        proc = subprocess.run(
            [str(RUN_TEST), str(test)],
            capture_output=True,
            text=True,
            timeout=timeout,
            cwd=str(REPO),
        )
    except subprocess.TimeoutExpired:
        return {"outcome": "timeout"}
    out = proc.stdout + "\n" + proc.stderr
    m = None
    for m in SUMMARY_RE.finditer(out):
        pass  # take the LAST summary (the file's own cleanupTests line)
    if not m:
        tail = "\n".join(out.splitlines()[-3:])[-200:]
        return {"outcome": "error", "tail": tail}
    total, passed, skipped, failed = (int(g) for g in m.groups())
    return {
        "outcome": "ran",
        "total": total,
        "passed": passed,
        "skipped": skipped,
        "failed": failed,
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--tier", default="all")
    ap.add_argument("--timeout", type=int, default=180)
    ap.add_argument("--json", default=None)
    ap.add_argument("stems", nargs="*")
    args = ap.parse_args()

    if not RUN_TEST.exists():
        print(f"run_test not built: {RUN_TEST}\n  build: cargo build --release -p tcl-vm --example run_test")
        return 2

    cref = load_c_ref()

    if args.stems:
        plan = [(s, "?") for s in args.stems]
    else:
        tiers = ["1", "2", "3"] if args.tier == "all" else [args.tier]
        plan = [(s, t) for t in tiers for s in TIERS[t]]

    rows = []
    n_match = n_close = n_far = n_err = 0
    print(f"{'stem':16} {'tier':4} {'C P/S/F':>14}  {'VM P/S/F':>14}  status")
    print("-" * 70)
    for stem, tier in plan:
        c = cref.get(stem)
        r = run_vm(stem, args.timeout)
        c_str = f"{c[1]}/{c[2]}/{c[3]}" if c else "(no ref)"
        if r["outcome"] != "ran":
            vm_str = r["outcome"].upper()
            status = "ERR" if r["outcome"] in ("error", "timeout") else r["outcome"]
            n_err += 1
        else:
            vm_str = f"{r['passed']}/{r['skipped']}/{r['failed']}"
            if c and (r["passed"], r["skipped"], r["failed"]) == (c[1], c[2], c[3]):
                status = "MATCH"
                n_match += 1
            elif c:
                dp = r["passed"] - c[1]
                df = r["failed"] - c[3]
                status = f"gap P{dp:+d} F{df:+d}"
                if c[0] and abs(dp) <= max(2, c[0] // 50):
                    n_close += 1
                else:
                    n_far += 1
            else:
                status = "no-ref"
                n_far += 1
        rows.append({"stem": stem, "tier": tier, "c": c, "vm": r, "status": status})
        print(f"{stem:16} {tier:4} {c_str:>14}  {vm_str:>14}  {status}")

    print("-" * 70)
    print(f"MATCH={n_match}  close={n_close}  far={n_far}  err/absent={n_err}  total={len(plan)}")
    if args.json:
        Path(args.json).write_text(json.dumps(rows, indent=2))
        print(f"wrote {args.json}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
