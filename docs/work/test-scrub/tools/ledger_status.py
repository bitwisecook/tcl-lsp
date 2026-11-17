#!/usr/bin/env python3
"""Progress of the test scrub, read from the ledgers' `status` column.

A status is one word from STATUSES; anything a session wants to add goes in a
`note` column. `--check` exits non-zero when a ledger uses another word.
"""

from __future__ import annotations

import argparse
import csv
import sys
from collections import Counter
from pathlib import Path

STATUSES = ("open", "repaired", "exposed", "fixed", "deleted", "blocked", "routed")
SIDE_FILES = ("clusters", "routed", "passing")
MEANING = {
    "open": "nothing done yet",
    "repaired": "the test was wrong, was corrected, and passes",
    "exposed": "the test was corrected and now fails on a product fault",
    "fixed": "the product was fixed and the test passes",
    "deleted": "merged into its survivor or retired",
    "blocked": "waits on a named row or session",
    "routed": "moved to another session's ledger",
}


def ledgers(directory: Path) -> list[Path]:
    return sorted(
        p for p in directory.glob("*.tsv") if not any(word in p.stem for word in SIDE_FILES)
    )


def read_statuses(path: Path) -> tuple[Counter[str], list[tuple[int, str]]]:
    """Counts by status, and the rows whose status is not in the vocabulary."""
    counts: Counter[str] = Counter()
    unknown: list[tuple[int, str]] = []
    with path.open(newline="", encoding="utf-8") as handle:
        rows = list(csv.reader(handle, delimiter="\t", quoting=csv.QUOTE_NONE))
    if not rows:
        return counts, unknown
    header = rows[0]
    column = header.index("status") if "status" in header else None
    for number, row in enumerate(rows[1:], start=2):
        if not any(cell.strip() for cell in row):
            continue
        value = row[column].strip() if column is not None and len(row) > column else ""
        value = value or "open"
        if value in STATUSES:
            counts[value] += 1
        else:
            counts["open"] += 1
            unknown.append((number, value))
    return counts, unknown


def table(directory: Path) -> tuple[str, int]:
    lines = [
        "| Ledger | " + " | ".join(STATUSES) + " | Rows |",
        "| --- | " + " | ".join("---:" for _ in STATUSES) + " | ---: |",
    ]
    total: Counter[str] = Counter()
    bad = 0
    for path in ledgers(directory):
        counts, unknown = read_statuses(path)
        bad += len(unknown)
        total.update(counts)
        cells = " | ".join(str(counts[s]) for s in STATUSES)
        lines.append(f"| {path.stem} | {cells} | {sum(counts.values())} |")
    cells = " | ".join(str(total[s]) for s in STATUSES)
    lines.append(f"| **all** | {cells} | {sum(total.values())} |")
    return "\n".join(lines), bad


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("directory", type=Path, help="the ledger directory")
    parser.add_argument("--check", action="store_true", help="fail on a status outside the vocabulary")
    args = parser.parse_args(argv)
    text, bad = table(args.directory)
    print(text)
    if args.check and bad:
        for path in ledgers(args.directory):
            for number, value in read_statuses(path)[1]:
                print(f"{path.name}:{number}: status {value[:60]!r} is not one of {', '.join(STATUSES)}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
