"""Aggregate per-test JSON records into ``tests/external/baseline-tcl9.json``.

Input: a file of concatenated JSON objects (either NDJSON or
jq-pretty).  Usage:

    uv run python scripts/tcltest_sweep/aggregate.py <results_path> [out_path]

``results_path`` defaults to ``/tmp/tcltest-sweep.ndjson`` (where
``run_all.sh`` writes by default); ``out_path`` defaults to
``tests/external/baseline-tcl9.json``.
"""

from __future__ import annotations

import json
import sys
from collections import Counter
from pathlib import Path


def parse_records(text: str) -> list[dict]:
    """Parse one-or-more concatenated JSON objects out of ``text``.

    Handles both NDJSON and jq-pretty shapes.  Skips malformed
    fragments rather than aborting.
    """
    entries: list[dict] = []
    decoder = json.JSONDecoder()
    pos = 0
    n = len(text)
    while pos < n:
        while pos < n and text[pos] in " \n\r\t":
            pos += 1
        if pos >= n:
            break
        try:
            obj, end = decoder.raw_decode(text, pos)
            entries.append(obj)
            pos = end
        except json.JSONDecodeError:
            nxt = text.find("{", pos + 1)
            if nxt < 0:
                break
            pos = nxt
    return entries


def main(argv: list[str]) -> int:
    results_path = Path(argv[1]) if len(argv) > 1 else Path("/tmp/tcltest-sweep.ndjson")
    out_path = Path(argv[2]) if len(argv) > 2 else Path("tests/external/baseline-tcl9.json")

    text = results_path.read_text()
    raw_entries = parse_records(text)
    # Dedupe by bundle name, keeping the LAST entry for each name
    # so an operator can re-run a subset (e.g. after fixing a
    # gap) without losing the update.
    by_name: dict[str, dict] = {}
    for e in raw_entries:
        by_name[e["name"]] = e
    entries = list(by_name.values())

    by_outcome = Counter(e["outcome"] for e in entries)
    passing = [e["name"] for e in entries if e["outcome"] == "pass"]
    failing = [e["name"] for e in entries if e["outcome"] == "fail"]
    trapping = [e["name"] for e in entries if e["outcome"] == "trap"]
    timing_out = [e["name"] for e in entries if e["outcome"] == "timeout"]
    no_summary = [e["name"] for e in entries if e["outcome"] == "no_summary"]
    unknowns = [e["name"] for e in entries if e["outcome"] == "unknown"]

    trap_reasons: Counter[str] = Counter()
    for e in entries:
        if e["outcome"] != "trap":
            continue
        trap = e.get("trap", "")
        if "unknown command:" in trap:
            cmd = trap.split("unknown command:", 1)[1].strip()[:60]
            trap_reasons[f"unknown-command:{cmd}"] += 1
        elif "cannot delete" in trap:
            trap_reasons["cannot-delete-current"] += 1
        elif "bad level" in trap:
            trap_reasons["bad-level"] += 1
        elif "unsupported" in trap.lower():
            trap_reasons[f"unsupported:{trap[:60]}"] += 1
        else:
            trap_reasons[trap[:80] or "(no trap line captured)"] += 1

    summary = {
        "total": len(entries),
        "by_outcome": dict(by_outcome),
        "passing": sorted(passing),
        "failing_with_test_errors": sorted(failing),
        "trapping": sorted(trapping),
        "timing_out": sorted(timing_out),
        "no_tcltest_summary": sorted(no_summary),
        "unknowns": sorted(unknowns),
        "trap_reason_buckets": dict(trap_reasons.most_common()),
        "entries": sorted(entries, key=lambda e: (e["outcome"], e["name"])),
    }

    # Print roll-up to stdout for CI / manual inspection.
    print(
        json.dumps(
            {
                "total": len(entries),
                "by_outcome": dict(by_outcome),
                "pass_count": len(passing),
                "top_trap_reasons": trap_reasons.most_common(10),
            },
            indent=2,
        )
    )

    out_path.write_text(json.dumps(summary, indent=2, sort_keys=False) + "\n")
    print(f"\nWrote {out_path}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
