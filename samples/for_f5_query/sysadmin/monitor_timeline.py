#!/usr/bin/env python3
"""Render an ASCII Gantt-style timeline of monitor up/down events.

Reads stdin produced by:

    f5 query --raw '
        f5log_load("logs/t1-a.log")[]
        | select(.module == "01340011" or .module == "01340012")
        | tsv(.timestamp,
              (sub(.message, "^.*member ", "") | sub(., " monitor.*$", "")),
              (if .module == "01340011" then "DOWN" else "UP" end))
    ' some.conf | grep -v '^#'

Each input line is TAB-separated: ``timestamp\tmember\tDOWN|UP``.

The timeline starts at the floor of the first event hour and uses a
configurable resolution (default 5 minutes per character).  Members
appear one per row with ``v`` for a DOWN transition, ``^`` for UP, and
``#`` for time the member was marked DOWN.
"""

from __future__ import annotations

import argparse
import collections
import datetime
import re
import sys

# Divisors of 60 — the resolutions where the hour-ruler logic produces
# a clean "one hour-label every N characters" layout.  Other values
# (7 / 11 / 13 …) leave the ruler unaligned with the data; >60 means
# fewer than one hour-tick per character which `60 // unit_minutes`
# rounds to zero and crashes ``range``.
_VALID_UNITS = (1, 2, 3, 4, 5, 6, 10, 12, 15, 20, 30, 60)

# Yearless syslog timestamps (``Mar 14 10:11:09``) parse against a
# constant leap-safe year so ``Feb 29`` doesn't raise.  Year only
# matters relative to other events in the same input; the renderer
# never displays it.  Inputs that straddle a Dec 31 → Jan 1 boundary
# are intentionally unsupported in v1 — operationally rare, and the
# log lines themselves carry no year, so reconstructing one would
# need a side-channel.  Pass ``--year`` to override the assumed year
# when investigating an older capture.
_LEAP_SAFE_YEAR = 2020


def parse_timestamp(s: str, year: int = _LEAP_SAFE_YEAR) -> datetime.datetime:
    """Parse a syslog ``Mon DD HH:MM:SS`` stamp; year is leap-safe."""
    return datetime.datetime.strptime(f"{year} {s}", "%Y %b %d %H:%M:%S")


def render(rows, unit_minutes: int = 5, year: int = _LEAP_SAFE_YEAR) -> str:
    by_member: dict[str, list[tuple[datetime.datetime, str]]] = collections.defaultdict(list)
    for ts, member, state in rows:
        by_member[member].append((parse_timestamp(ts, year), state))

    if not by_member:
        return "(no events)"

    flat = [t for events in by_member.values() for t, _ in events]
    start = min(flat).replace(
        minute=(min(flat).minute // unit_minutes) * unit_minutes,
        second=0,
    )
    end = max(flat).replace(minute=0, second=0) + datetime.timedelta(hours=1)
    width = int((end - start).total_seconds() // (unit_minutes * 60)) + 1

    def col(dt: datetime.datetime) -> int:
        return int((dt - start).total_seconds() // (unit_minutes * 60))

    # Hour ruler.  ``unit_minutes`` is validated by argparse to be a
    # divisor of 60, so ``60 // unit_minutes`` is always >= 1.
    ruler = [" "] * (22 + width)
    minutes_per_hour_char = 60 // unit_minutes
    for h_col in range(0, width, minutes_per_hour_char):
        h = (start + datetime.timedelta(minutes=h_col * unit_minutes)).hour
        s = f"{h:02d}"
        for i, c in enumerate(s):
            if 22 + h_col + i < len(ruler):
                ruler[22 + h_col + i] = c

    out: list[str] = []
    out.append(f"members down/up over time (1 char = {unit_minutes} min)")
    out.append("".join(ruler).rstrip())
    out.append(" " * 22 + "+" + "-" * (width - 1))

    for member in sorted(by_member):
        state_row = [" "] * width  # default UP
        cur = "UP"
        prev_col = 0
        for dt, s in sorted(by_member[member]):
            cc = col(dt)
            for i in range(prev_col, cc):
                state_row[i] = " " if cur == "UP" else "#"
            cur = s
            state_row[cc] = "v" if s == "DOWN" else "^"
            prev_col = cc + 1
        for i in range(prev_col, width):
            state_row[i] = " " if cur == "UP" else "#"
        name = re.sub(r"^/Common/", "", member)[:20]
        out.append(f"{name:20s}  |" + "".join(state_row).rstrip())

    return "\n".join(out)


def _unit_minutes(value: str) -> int:
    """argparse type: a positive integer divisor of 60."""
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
    print(render(rows, args.unit_minutes, args.year))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
