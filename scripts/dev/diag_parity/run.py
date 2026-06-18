#!/usr/bin/env python3
"""Differential `diag` parity harness: Rust `tcl` vs Python `tooling.tcl.main`.

Runs both diagnostic engines over a corpus of Tcl / iRule snippets across the
{tcl8.6, f5-irules} dialects, then classifies every per-diagnostic divergence
and prints a ranked frequency table of ``(code x divergence-kind)``.

Ground truth is the Python CLI (``python -m tooling.tcl.main diag``).  The Rust
binary under test is ``target/debug/tcl``.  This table is the regression oracle
for the analyser-parity work that unblocks ``find-legacy`` and ``minimize``.

Divergence kinds
----------------
MISSING_FIRE    code emitted by Python but not Rust (py-only).
EXTRA_FIRE      code emitted by Rust but not Python (rust-only).
WRONG_POSITION  same code, paired occurrence at a different line/column.
WRONG_MESSAGE   same code+position, different message text.
WRONG_SEVERITY  same code+position, different severity tier.
WRONG_ORDER     identical diagnostic multiset, different emission sequence.

Usage
-----
    python scripts/dev/diag_parity/run.py                 # committed corpus
    python scripts/dev/diag_parity/run.py --tcl-tests 40  # + sampled .test files
    python scripts/dev/diag_parity/run.py --json report.json
    python scripts/dev/diag_parity/run.py --show-codes W100,IRULE2001
"""

from __future__ import annotations

import argparse
import collections
import json
import os
import random
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
RUST_BIN = REPO / "target" / "debug" / "tcl"
VENV_PY = REPO / ".venv" / "bin" / "python"
CORPUS = Path(__file__).resolve().parent / "corpus"
DIALECTS = ("tcl8.6", "f5-irules")

Diag = tuple[str, int, int, str, str]  # (code, line, column, severity, message)

KINDS = (
    "MISSING_FIRE",
    "EXTRA_FIRE",
    "WRONG_POSITION",
    "WRONG_MESSAGE",
    "WRONG_SEVERITY",
    "WRONG_ORDER",
)


def _parse_report(stdout: str) -> list[Diag] | None:
    try:
        report = json.loads(stdout)
    except json.JSONDecodeError:
        return None
    diags: list[Diag] = []
    for entry in report:
        for d in entry.get("diagnostics", []):
            diags.append(
                (
                    d.get("code") or "",
                    int(d["line"]),
                    int(d["column"]),
                    d.get("severity") or "",
                    d.get("message") or "",
                )
            )
    return diags


def _run(cmd: list[str], env: dict[str, str] | None = None) -> list[Diag] | None:
    proc = subprocess.run(
        cmd, capture_output=True, text=True, env=env, cwd=str(REPO)
    )
    return _parse_report(proc.stdout)


def run_python(path: Path, dialect: str) -> list[Diag] | None:
    env = {**os.environ, "PYTHONPATH": str(REPO)}
    py = str(VENV_PY) if VENV_PY.exists() else sys.executable
    return _run(
        [py, "-m", "tooling.tcl.main", "diag", "--dialect", dialect, str(path), "--json"],
        env,
    )


def run_rust(path: Path, dialect: str) -> list[Diag] | None:
    return _run([str(RUST_BIN), "diag", "--dialect", dialect, str(path), "--json"])


def classify(py: list[Diag], rust: list[Diag]) -> list[tuple[str, str, dict]]:
    """Return ``(code, kind, detail)`` divergences between two diagnostic lists.

    The matching is layered: exact tuples are removed order-insensitively
    first; the leftovers are paired per-code to surface position / message /
    severity drift; whatever is still unpaired is a missing or extra fire.  A
    purely-reordered (multiset-equal) pair is reported as WRONG_ORDER.
    """
    out: list[tuple[str, str, dict]] = []
    pyc = collections.Counter(py)
    rc = collections.Counter(rust)
    common = pyc & rc
    py_left = list((pyc - common).elements())
    rust_left = list((rc - common).elements())

    if not py_left and not rust_left:
        if py != rust:
            py_codes = [t[0] for t in py]
            rust_codes = [t[0] for t in rust]
            first_py = {c: py_codes.index(c) for c in set(py_codes)}
            first_rust = {c: rust_codes.index(c) for c in set(rust_codes)}
            for code in set(first_py) | set(first_rust):
                if first_py.get(code) != first_rust.get(code):
                    out.append((code, "WRONG_ORDER", {}))
        return out

    by_py: dict[str, list[Diag]] = collections.defaultdict(list)
    for t in py_left:
        by_py[t[0]].append(t)
    by_rust: dict[str, list[Diag]] = collections.defaultdict(list)
    for t in rust_left:
        by_rust[t[0]].append(t)

    for code in set(by_py) | set(by_rust):
        pl = sorted(by_py[code])
        rl = sorted(by_rust[code])
        paired = min(len(pl), len(rl))
        for i in range(paired):
            _, pline, pcol, psev, pmsg = pl[i]
            _, rline, rcol, rsev, rmsg = rl[i]
            same_pos = (pline, pcol) == (rline, rcol)
            detail = {
                "py": (pline, pcol, psev, pmsg),
                "rust": (rline, rcol, rsev, rmsg),
            }
            if not same_pos:
                out.append((code, "WRONG_POSITION", detail))
            if same_pos and pmsg != rmsg:
                out.append((code, "WRONG_MESSAGE", detail))
            if same_pos and pmsg == rmsg and psev != rsev:
                out.append((code, "WRONG_SEVERITY", detail))
        for i in range(paired, len(pl)):
            out.append((code, "MISSING_FIRE", {"py": pl[i][1:]}))
        for i in range(paired, len(rl)):
            out.append((code, "EXTRA_FIRE", {"rust": rl[i][1:]}))
    return out


def corpus_files(extra_tcl_tests: int, stage: Path) -> list[Path]:
    """Collect corpus paths; the CLIs only accept Tcl/iRule extensions, so
    sampled ``*.test`` files are staged into *stage* renamed to ``*.tcl``."""
    files = sorted(CORPUS.glob("*.tcl")) + sorted(CORPUS.glob("*.irule"))
    if extra_tcl_tests > 0:
        pool = sorted((REPO / "tmp").glob("tcl*/tests/*.test"))
        rng = random.Random(1234)
        rng.shuffle(pool)
        for src in pool[:extra_tcl_tests]:
            dst = stage / f"{src.parent.parent.name}__{src.stem}.tcl"
            shutil.copyfile(src, dst)
            files.append(dst)
    return files


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--tcl-tests",
        type=int,
        default=0,
        metavar="N",
        help="Also sample N real .test files from tmp/tcl*/tests/.",
    )
    ap.add_argument(
        "--dialects",
        default=",".join(DIALECTS),
        help="Comma-separated dialects to run (default: tcl8.6,f5-irules).",
    )
    ap.add_argument("--json", metavar="FILE", help="Write the full report as JSON.")
    ap.add_argument(
        "--show-codes",
        default="",
        help="Comma-separated codes to dump per-occurrence detail for.",
    )
    args = ap.parse_args()

    if not RUST_BIN.exists():
        print(
            f"error: rust binary not found at {RUST_BIN}; run "
            "`cargo build -p tcl-cli --bin tcl` first",
            file=sys.stderr,
        )
        return 2

    dialects = [d.strip() for d in args.dialects.split(",") if d.strip()]
    show = {c.strip().upper() for c in args.show_codes.split(",") if c.strip()}
    stage = Path(tempfile.mkdtemp(prefix="diag_parity_"))
    files = corpus_files(args.tcl_tests, stage)

    table: collections.Counter[tuple[str, str]] = collections.Counter()
    detail_log: list[dict] = []
    per_file: list[dict] = []
    skipped = 0

    for path in files:
        for dialect in dialects:
            py = run_python(path, dialect)
            rust = run_rust(path, dialect)
            if py is None or rust is None:
                skipped += 1
                continue
            divergences = classify(py, rust)
            for code, kind, det in divergences:
                table[(code, kind)] += 1
                if code in show:
                    detail_log.append(
                        {
                            "file": str(path.relative_to(REPO))
                            if path.is_relative_to(REPO)
                            else str(path),
                            "dialect": dialect,
                            "code": code,
                            "kind": kind,
                            "detail": det,
                        }
                    )
            if divergences:
                per_file.append(
                    {
                        "file": str(path.relative_to(REPO))
                        if path.is_relative_to(REPO)
                        else str(path),
                        "dialect": dialect,
                        "divergences": [(c, k) for c, k, _ in divergences],
                    }
                )

    print(f"corpus: {len(files)} files x {len(dialects)} dialect(s)")
    if skipped:
        print(f"skipped {skipped} runs (non-JSON / crash on one engine)")
    print()
    print("ranked divergence table  (code x kind : count)")
    print("=" * 52)
    if not table:
        print("  no divergences \U0001F389")
    else:
        width = max(len(c) for c, _ in table)
        for (code, kind), count in sorted(
            table.items(), key=lambda kv: (-kv[1], kv[0][0], kv[0][1])
        ):
            print(f"  {code:<{width}}  {kind:<14} {count:>5}")
    print()
    print("totals by kind")
    print("-" * 32)
    by_kind: collections.Counter[str] = collections.Counter()
    for (_, kind), count in table.items():
        by_kind[kind] += count
    for kind in KINDS:
        print(f"  {kind:<14} {by_kind.get(kind, 0):>5}")

    if show and detail_log:
        print()
        print(f"per-occurrence detail for {sorted(show)}")
        print("-" * 32)
        for item in detail_log:
            print(
                f"  [{item['dialect']}] {item['code']} {item['kind']} "
                f"{item['file']}"
            )
            print(f"      {item['detail']}")

    if args.json:
        Path(args.json).write_text(
            json.dumps(
                {
                    "table": [
                        {"code": c, "kind": k, "count": n}
                        for (c, k), n in table.most_common()
                    ],
                    "per_file": per_file,
                },
                indent=2,
            )
        )
        print(f"\nwrote {args.json}")

    shutil.rmtree(stage, ignore_errors=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
