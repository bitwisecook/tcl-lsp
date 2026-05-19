#!/usr/bin/env python3
"""Render an ASCII Gantt-style timeline of monitor up/down events.

Thin shim around the canonical implementation in
:mod:`dialects.f5.query.renderers.gantt`.  Preserved so the documented
``| python3 sysadmin/monitor_timeline.py`` pipeline keeps working — new
callers should prefer ``f5 query --render gantt`` or, from Python,
``from dialects.f5.query import Query; Query(...).run(...).render("gantt")``.

Reads stdin produced by:

    f5 query --raw '
        f5log_load("logs/t1-a.log")[]
        | select(.module == "01340011" or .module == "01340012")
        | tsv(.timestamp,
              (sub(.message, "^.*member ", "") | sub(., " monitor.*$", "")),
              (if .module == "01340011" then "DOWN" else "UP" end))
    ' some.conf | grep -v '^#'

Each input line is TAB-separated: ``timestamp\\tmember\\tDOWN|UP``.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

# Allow running directly from the samples directory without an install
# — extend sys.path to the repo root so ``dialects.f5.query.renderers``
# resolves.  The shim is dual-purposed: it works both as
# ``uv run python samples/for_f5_query/sysadmin/monitor_timeline.py``
# (from the repo) and as a standalone script users have copied out.
_REPO_ROOT = Path(__file__).resolve().parents[3]
if str(_REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(_REPO_ROOT))

from dialects.f5.query.renderers.gantt import render_gantt  # noqa: E402

_VALID_UNITS = (1, 2, 3, 4, 5, 6, 10, 12, 15, 20, 30, 60)
_LEAP_SAFE_YEAR = 2020


def _unit_minutes(value: str) -> int:
    try:
        ivalue = int(value)
    except ValueError as exc:
        raise argparse.ArgumentTypeError(f"{value!r} is not an integer") from exc
    if ivalue not in _VALID_UNITS:
        raise argparse.ArgumentTypeError(
            f"--unit-minutes must be a positive divisor of 60 "
            f"(one of {', '.join(str(u) for u in _VALID_UNITS)}); got {ivalue}"
        )
    return ivalue


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Render an ASCII Gantt-style timeline of monitor up/down events.",
        epilog=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--unit-minutes",
        type=_unit_minutes,
        default=5,
        help="Minutes per output character; must be a positive divisor of 60 (default: 5).",
    )
    parser.add_argument(
        "--year",
        type=int,
        default=_LEAP_SAFE_YEAR,
        help=(
            "Year to assume for yearless syslog timestamps. Defaults to a "
            "leap year so 'Feb 29 ...' lines parse. Only affects relative "
            "ordering — the year is never displayed."
        ),
    )
    args = parser.parse_args()
    rows: list[tuple[str, str, str]] = []
    for line in sys.stdin:
        line = line.rstrip("\n")
        if not line or line.startswith("#"):
            continue
        parts = line.split("\t")
        if len(parts) >= 3:
            rows.append((parts[0], parts[1], parts[2]))
    sys.stdout.write(render_gantt(rows, unit_minutes=args.unit_minutes, year=args.year))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
