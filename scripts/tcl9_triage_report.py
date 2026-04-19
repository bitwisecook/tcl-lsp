#!/usr/bin/env python3
"""Rewrite the Tcl 9 triage table from a harness JSON report.

Usage:
    python scripts/tcl9_triage_report.py tmp/tcl9-report.json [--kcs PATH]

Reads the JSON artefact emitted by ``make test-tcl9`` (see
``tests/conftest.py`` ``pytest_sessionfinish``) and rewrites the region
between ``<!-- TRIAGE-BEGIN -->`` and ``<!-- TRIAGE-END -->`` markers
inside ``docs/kcs/kcs-tcl9-triage.md``. Everything outside the fenced
region is left untouched so hand-maintained prose survives the refresh.

The JSON schema is a list of dicts, one per pytest test function
(compile stage and run stage emit separate entries). Fields in use:

    file, subsystem, stage, status, total, passed, skipped, failed,
    first_failing_test, trap_site, stderr_tail, category

Stages emit ordering: compile → run. The triage table aggregates to one
row per file by preferring the run-stage result when present, falling
back to the compile-stage result. A file that only has a compile entry
(because the harness fail-fasted at compile time) keeps stage=compile.
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

BEGIN = "<!-- TRIAGE-BEGIN -->"
END = "<!-- TRIAGE-END -->"

_CATEGORY_ORDER = {"A": 0, "B": 1, "C": 2, "E": 3, "D": 4, "pass": 5}


def _aggregate(entries: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Collapse compile + run entries into one row per file.

    Prefer the run entry when it exists. When only compile is present,
    use that (it means the file never made it to the run stage).
    """
    by_file: dict[str, dict[str, dict[str, Any]]] = defaultdict(dict)
    for entry in entries:
        file = entry.get("file")
        stage = entry.get("stage", "run")
        if not file:
            continue
        by_file[file][stage] = entry

    rows: list[dict[str, Any]] = []
    for file, stages in by_file.items():
        chosen = stages.get("run") or stages.get("compile")
        if chosen is None:
            continue
        rows.append(chosen)
    return rows


def _format_row(row: dict[str, Any]) -> str:
    file = row.get("file", "")
    subsystem = row.get("subsystem", "")
    stage = row.get("stage", "")
    status = row.get("status", "")
    category = row.get("category", "")
    total = row.get("total", 0) or 0
    passed = row.get("passed", 0) or 0
    failed = row.get("failed", 0) or 0
    first_fail = row.get("first_failing_test") or ""
    trap = (row.get("trap_site") or "").replace("|", "\\|").replace("\n", " ")
    if len(trap) > 80:
        trap = trap[:77] + "…"
    counts = f"{passed}/{total}" if total else "—"
    return (
        f"| `{file}` | {subsystem} | {category} | {stage} | {status} | "
        f"{counts} | {failed} | `{first_fail}` | {trap} |"
    )


def _render_table(entries: list[dict[str, Any]]) -> str:
    rows = _aggregate(entries)
    rows.sort(
        key=lambda r: (
            _CATEGORY_ORDER.get(r.get("category", "") or "", 99),
            -(r.get("failed") or 0),
            r.get("subsystem", ""),
            r.get("file", ""),
        )
    )

    counts: dict[str, int] = defaultdict(int)
    for r in rows:
        counts[r.get("category", "?")] += 1

    summary = (
        f"Totals: pass={counts.get('pass', 0)}, "
        f"A={counts.get('A', 0)}, B={counts.get('B', 0)}, "
        f"C={counts.get('C', 0)}, D={counts.get('D', 0)}, "
        f"E={counts.get('E', 0)}, files={len(rows)}."
    )

    lines = [
        summary,
        "",
        "| File | Subsystem | Category | Stage | Status | Passed/Total | Failed | First failing | Trap site |",
        "|---|---|---|---|---|---|---|---|---|",
    ]
    lines.extend(_format_row(r) for r in rows)
    return "\n".join(lines)


def _rewrite_fenced(markdown: str, inner: str) -> str:
    if BEGIN not in markdown or END not in markdown:
        raise SystemExit(f"triage KCS missing {BEGIN!r}/{END!r} fence; cannot rewrite")
    before, _, rest = markdown.partition(BEGIN)
    _old, _, after = rest.partition(END)
    return f"{before}{BEGIN}\n\n{inner}\n\n{END}{after}"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", type=Path, help="Path to tcl9-report.json")
    parser.add_argument(
        "--kcs",
        type=Path,
        default=Path("docs/kcs/kcs-tcl9-triage.md"),
        help="Path to the triage KCS file (default: docs/kcs/kcs-tcl9-triage.md)",
    )
    args = parser.parse_args(argv)

    if not args.report.exists():
        print(f"Report not found: {args.report}", file=sys.stderr)
        return 2
    if not args.kcs.exists():
        print(f"KCS file not found: {args.kcs}", file=sys.stderr)
        return 2

    entries = json.loads(args.report.read_text(encoding="utf-8"))
    if not isinstance(entries, list):
        print(f"Report must be a JSON list, got {type(entries).__name__}", file=sys.stderr)
        return 2

    table = _render_table(entries)
    markdown = args.kcs.read_text(encoding="utf-8")
    updated = _rewrite_fenced(markdown, table)
    if updated != markdown:
        args.kcs.write_text(updated, encoding="utf-8")
    print(f"Wrote triage table ({len(entries)} entries) to {args.kcs}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
