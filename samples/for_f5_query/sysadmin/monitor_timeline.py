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


def parse_timestamp(s: str) -> datetime.datetime:
    """``Mar 14 10:11:09`` -> naive datetime."""
    return datetime.datetime.strptime(s, "%b %d %H:%M:%S")


def render(rows, unit_minutes: int = 5) -> str:
    by_member: dict[str, list[tuple[datetime.datetime, str]]] = collections.defaultdict(list)
    for ts, member, state in rows:
        by_member[member].append((parse_timestamp(ts), state))

    if not by_member:
        return "(no events)"

    flat = [t for events in by_member.values() for t, _ in events]
    start = min(flat).replace(minute=(min(flat).minute // unit_minutes) * unit_minutes, second=0)
    end = max(flat).replace(minute=0, second=0) + datetime.timedelta(hours=1)
    width = int((end - start).total_seconds() // (unit_minutes * 60)) + 1

    def col(dt: datetime.datetime) -> int:
        return int((dt - start).total_seconds() // (unit_minutes * 60))

    # Hour ruler
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


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--unit-minutes", type=int, default=5)
    args = parser.parse_args()
    rows: list[tuple[str, str, str]] = []
    for line in sys.stdin:
        line = line.rstrip("\n")
        if not line or line.startswith("#"):
            continue
        parts = line.split("\t")
        if len(parts) >= 3:
            rows.append((parts[0], parts[1], parts[2]))
    print(render(rows, args.unit_minutes))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
