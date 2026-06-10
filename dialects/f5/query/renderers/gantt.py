"""``gantt`` renderer — ASCII Gantt of timestamped state transitions.

Replaces the standalone ``samples/for_f5_query/sysadmin/monitor_timeline.py``
script.  Accepts the same row shape ``tsv(.timestamp, .label, .state)``
produces, plus a few friendlier alternatives:

- a string — split on newlines, each non-blank non-``#`` line split on
  ``\\t`` into three cells (the original "pipe TSV from ``f5 query --raw``"
  shape);
- a list of 3-tuples / 3-lists ``(timestamp, label, state)``;
- a list of dicts with ``timestamp`` / ``label`` / ``state`` (or
  ``member`` / ``state``) keys — what an object-literal query like
  ``{timestamp: .timestamp, label: ..., state: ...}`` produces.

Options (via ``--render-opt KEY=VALUE``):

- ``unit-minutes`` — minutes per output character, a positive divisor of
  60 (default 5).
- ``year`` — year to assume for yearless syslog timestamps (default a
  leap-safe year so ``Feb 29`` parses).
"""

from __future__ import annotations

import collections
import datetime
import re
from typing import Any, Iterable

from ..errors import RendererError
from ..values import ObjectRef, PathRef, Stream
from . import renderer

_VALID_UNITS = (1, 2, 3, 4, 5, 6, 10, 12, 15, 20, 30, 60)
_LEAP_SAFE_YEAR = 2020


@renderer(
    "gantt",
    summary="ASCII Gantt-style timeline of (timestamp, label, state) transitions.",
    accepts="rows of (timestamp, label, state) — TSV string, 3-tuples, or dicts",
    details=(
        "Lifted from samples/for_f5_query/sysadmin/monitor_timeline.py.  "
        "Renders one row per distinct label, with ``v`` for the DOWN "
        "transition, ``^`` for UP, and ``#`` for the spans the label was "
        "marked down.  Use --render-opt unit-minutes=N (a divisor of 60) "
        "to change the time resolution."
    ),
)
def _render_gantt(values: list[Any], **opts: Any) -> str:
    unit_minutes = _parse_unit_minutes(opts.get("unit-minutes", opts.get("unit_minutes", 5)))
    year = _parse_year(opts.get("year", _LEAP_SAFE_YEAR))
    rows = list(_coerce_rows(values))
    return render_gantt(rows, unit_minutes=unit_minutes, year=year)


def render_gantt(
    rows: Iterable[tuple[str, str, str]],
    *,
    unit_minutes: int = 5,
    year: int = _LEAP_SAFE_YEAR,
) -> str:
    """Render *rows* as an ASCII Gantt chart.

    Public-API entry kept module-level so external callers can use the
    renderer without going through the registry — useful for tests and
    for the legacy ``monitor_timeline.py`` shim.
    """
    by_member: dict[str, list[tuple[datetime.datetime, str]]] = collections.defaultdict(list)
    for ts, member, state in rows:
        by_member[member].append((_parse_timestamp(ts, year), state))

    if not by_member:
        return "(no events)\n"

    flat = [t for events in by_member.values() for t, _ in events]
    start = min(flat).replace(
        minute=(min(flat).minute // unit_minutes) * unit_minutes,
        second=0,
    )
    end = max(flat).replace(minute=0, second=0) + datetime.timedelta(hours=1)
    width = int((end - start).total_seconds() // (unit_minutes * 60)) + 1

    def col(dt: datetime.datetime) -> int:
        return int((dt - start).total_seconds() // (unit_minutes * 60))

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
        state_row = [" "] * width
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

    return "\n".join(out) + "\n"


def _parse_timestamp(s: str, year: int) -> datetime.datetime:
    try:
        return datetime.datetime.strptime(f"{year} {s}", "%Y %b %d %H:%M:%S")
    except ValueError as exc:
        raise RendererError(f"gantt: cannot parse timestamp {s!r}: {exc}") from exc


def _parse_unit_minutes(value: Any) -> int:
    try:
        ivalue = int(value)
    except (TypeError, ValueError) as exc:
        raise RendererError(f"gantt: unit-minutes must be an integer (got {value!r})") from exc
    if ivalue not in _VALID_UNITS:
        valid = ", ".join(str(u) for u in _VALID_UNITS)
        raise RendererError(
            f"gantt: unit-minutes must be a positive divisor of 60 (one of {valid}); got {ivalue}"
        )
    return ivalue


def _parse_year(value: Any) -> int:
    try:
        return int(value)
    except (TypeError, ValueError) as exc:
        raise RendererError(f"gantt: year must be an integer (got {value!r})") from exc


def _coerce_rows(values: list[Any]) -> Iterable[tuple[str, str, str]]:
    """Yield ``(timestamp, label, state)`` triples from a heterogeneous list.

    Accepts:

    - a single string — split on lines, each non-blank non-``#`` line
      split on ``\\t``.
    - a list whose first element is a string and whose total length is
      3 — treat the whole list as one row (the natural shape when a
      caller wraps a single ``tsv(...)`` result in ``[...]``).
    - a list of 3-tuples / 3-element lists / streams.
    - a list of dicts with ``timestamp`` / ``label`` / ``state`` keys
      (or ``member`` instead of ``label``).
    - a list of TSV strings — one row per element.

    Empty lines and comment lines (``#``-prefixed) are dropped silently
    so the canonical
    ``f5 query --raw '...' | grep -v '^#' | gantt`` pipeline works
    without the ``grep``.
    """
    for v in values:
        if isinstance(v, Stream):
            yield from _coerce_rows(list(v))
            continue
        if isinstance(v, str):
            yield from _rows_from_text(v)
            continue
        if isinstance(v, (list, tuple)):
            items = list(v)
            if len(items) == 3 and all(isinstance(c, (str, int, float, PathRef)) for c in items):
                yield (_cell(items[0]), _cell(items[1]), _cell(items[2]))
                continue
            yield from _coerce_rows(items)
            continue
        if isinstance(v, dict):
            ts = v.get("timestamp") or v.get("ts") or v.get("time")
            label = v.get("label") or v.get("member") or v.get("name")
            state = v.get("state")
            if ts is None or label is None or state is None:
                raise RendererError(
                    "gantt: dict rows must carry 'timestamp', "
                    "'label' (or 'member'), and 'state' keys; got "
                    f"{sorted(v.keys())!r}"
                )
            yield (_cell(ts), _cell(label), _cell(state))
            continue
        if isinstance(v, ObjectRef):
            raise RendererError(
                "gantt: ObjectRef rows are not supported; project the "
                "(timestamp, label, state) fields explicitly via "
                "`tsv(...)` or `{timestamp, label, state}`"
            )
        raise RendererError(f"gantt: unsupported row shape {type(v).__name__}")


def _rows_from_text(text: str) -> Iterable[tuple[str, str, str]]:
    for line in text.splitlines():
        line = line.rstrip("\r")
        if not line or line.startswith("#"):
            continue
        parts = line.split("\t")
        if len(parts) < 3:
            raise RendererError(
                f"gantt: expected at least 3 tab-separated cells, got {len(parts)} in {line!r}"
            )
        yield (parts[0], parts[1], parts[2])


def _cell(v: Any) -> str:
    if isinstance(v, PathRef):
        return v.full_path
    if v is None:
        return ""
    return str(v)
