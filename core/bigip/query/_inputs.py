"""Structured-input parsers for the f5 query DSL.

Side-inputs (``--input-json`` / ``--input-jsonl`` / ``--input-csv`` /
``--input-f5log``) bind a file's parsed content to a DSL variable.
This module hosts the parsers shared between the CLI plumbing and
the equivalent builtins (``json_load`` / ``jsonl_load`` /
``csv_load`` / ``f5log_load``) so a query can mix CLI-loaded inputs
and in-query loads from the same code path.

Every parser:

- Is offline-safe — pure local file IO / in-memory parsing, no
  network.
- Returns a Python value the DSL can navigate naturally (typically
  a list of dicts).
- Raises a clear, actionable error on malformed input instead of
  producing a half-parsed shape.
"""

from __future__ import annotations

import csv as _csv
import io
import json as _json
import re
from dataclasses import dataclass, field
from typing import Any, Iterable

# ---------------------------------------------------------------------------
# JSON Lines (NDJSON)
# ---------------------------------------------------------------------------


def parse_jsonl(text: str, *, source: str = "<jsonl>") -> list[Any]:
    """Parse newline-delimited JSON into a list of values.

    Blank lines are ignored; non-blank lines must each be a single
    JSON value.  Returns one Python value per line in file order.
    Errors quote the line number so the user can find the bad line
    in a large dump without bisecting.
    """
    out: list[Any] = []
    for line_no, raw in enumerate(text.splitlines(), start=1):
        line = raw.strip()
        if not line:
            continue
        try:
            out.append(_json.loads(line))
        except _json.JSONDecodeError as exc:
            raise InputError(
                f"{source}: line {line_no}: invalid JSON ({exc.msg} at col {exc.colno})"
            ) from exc
    return out


# ---------------------------------------------------------------------------
# CSV
# ---------------------------------------------------------------------------


def parse_csv(
    text: str,
    *,
    headers: list[str] | None = None,
    source: str = "<csv>",
) -> list[dict[str, Any]]:
    """Parse CSV text into a list of dicts.

    When *headers* is ``None`` the first row of *text* names the
    columns (jq's natural shape).  When *headers* is supplied,
    every row of *text* is a data row and the supplied list names
    each column — handy for CSVs without a header row.

    Empty / whitespace-only rows are skipped.  Values are returned
    as strings; the caller can coerce them with ``str()`` /
    arithmetic-on-string (the DSL's ``+`` coerces) if numeric
    interpretation is wanted.  Missing trailing columns become
    empty strings; extras land in a ``_extra`` list.
    """
    reader = _csv.reader(io.StringIO(text))
    rows = [row for row in reader if any(cell.strip() for cell in row)]
    if not rows:
        return []
    if headers is None:
        headers = [h.strip() for h in rows[0]]
        data_rows: Iterable[list[str]] = rows[1:]
    else:
        data_rows = rows
    if not headers:
        raise InputError(f"{source}: no header columns — supply --input-csv NAME=PATH:hdr1,hdr2,…")
    out: list[dict[str, Any]] = []
    width = len(headers)
    for line_no, row in enumerate(data_rows, start=2 if headers is None else 1):
        record: dict[str, Any] = {}
        for i, name in enumerate(headers):
            record[name] = row[i] if i < len(row) else ""
        if len(row) > width:
            record["_extra"] = list(row[width:])
        out.append(record)
    return out


# ---------------------------------------------------------------------------
# F5 BIG-IP logs
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class F5LogEvent:
    """One parsed BIG-IP log line.

    Fields the parser can recover from the typical formats
    (``/var/log/ltm``, ``/var/log/tmm``, audit, hsl):

    - ``timestamp``  : ISO-ish "Mon DD HH:MM:SS" or RFC3339 when
      present.
    - ``host``       : the hostname segment (may be ``""`` for
      stripped logs).
    - ``severity``   : ``info`` / ``notice`` / ``warning`` /
      ``err`` / ``crit`` / ``alert`` / ``emerg`` / ``debug`` /
      ``""`` when absent.
    - ``daemon``     : the process name (``tmm`` / ``mcpd`` /
      ``bigd`` / ``logger`` / …).
    - ``pid``        : integer or ``None``.
    - ``code``       : F5 message code (``"01230140:6"``) or ``""``.
    - ``module``     : the module half of *code* (``"01230140"``)
      or ``""``.
    - ``level``      : the severity-byte half of *code* (``6``) or
      ``None``.
    - ``message``    : the post-code text.

    Returned as a dict from :func:`parse_f5log` (one entry per
    line) so the query DSL can navigate fields the usual way.
    """

    timestamp: str = ""
    host: str = ""
    severity: str = ""
    daemon: str = ""
    pid: int | None = None
    code: str = ""
    module: str = ""
    level: int | None = None
    message: str = ""
    raw: str = ""


_F5_SEVERITIES = frozenset(
    {"emerg", "alert", "crit", "err", "warning", "notice", "info", "debug"}
)

# Only one tiny regex remains — to strip the optional syslog PRI
# prefix ``<NNN>`` at the very start of a line.  Everything else is
# a deterministic tokeniser.  PRI is well-defined (``<`` digits ``>``)
# and the tokeniser would have to special-case it anyway, so the
# regex is the simplest spelling.
_PRI_PREFIX = re.compile(r"^<\d{1,3}>")


def parse_f5log(text: str, *, source: str = "<f5log>") -> list[dict[str, Any]]:
    """Parse a BIG-IP log file into a list of event dicts.

    The parser is a deterministic tokeniser — no regex tower.
    Lines that don't match the canonical syslog shape land as a
    single-field event with ``raw`` set and the other fields blank,
    so downstream queries can still see the original text.

    Format families handled:

    - **Classic syslog** (``Mon DD HH:MM:SS host info daemon[pid]: …``)
    - **RFC3164 with PRI** (``<27>Mon DD …``) — PRI stripped.
    - **F5 tmm-style with code** (``daemon[pid]: 01230140:6: …``):
      the ``XXXXXXXX:N`` code splits into ``module`` + ``level`` so
      audits can filter by module without sub-parsing the message.
    - **Audit** (``logger[pid]: 01070417:6: AUDIT - …``): no
      special-casing — the ``AUDIT`` marker stays in ``message``,
      which is what most pipelines actually want to grep on.

    Anything unrecognised is stored verbatim under ``raw`` so the
    operator never silently loses a line.  ``source`` is the
    label used in error messages (the URI or path).
    """
    events: list[dict[str, Any]] = []
    for line_no, raw_line in enumerate(text.splitlines(), start=1):
        if not raw_line.strip():
            continue
        event = _parse_one_f5log(raw_line.rstrip("\n"))
        events.append(_event_to_dict(event))
    _ = source  # reserved for future error reporting
    return events


def _parse_one_f5log(line: str) -> F5LogEvent:
    """Tokenise a single line into an :class:`F5LogEvent`.

    The strategy is char-by-char-ish: peel optional fragments off
    the head (PRI, timestamp, host, severity, daemon[pid]), and
    treat everything after the ``:`` as message text — splitting
    out a leading F5 code if present.

    The parser is forgiving: any peeling step that fails leaves
    the remainder as ``message`` and stops, so a truly unfamiliar
    line still surfaces in the output (with ``raw`` set) instead
    of raising.
    """
    raw = line
    body = _PRI_PREFIX.sub("", line, count=1)
    timestamp, body = _take_timestamp(body)
    if timestamp is None:
        return F5LogEvent(raw=raw, message=line)
    host, body = _take_token(body)
    severity, body = _take_severity(body)
    daemon, pid, body = _take_daemon_pid(body)
    body = body.lstrip()
    if body.startswith(":"):
        body = body[1:].lstrip()
    code, module, level, body = _take_f5_code(body)
    return F5LogEvent(
        timestamp=timestamp,
        host=host,
        severity=severity,
        daemon=daemon,
        pid=pid,
        code=code,
        module=module,
        level=level,
        message=body,
        raw=raw,
    )


def _take_timestamp(text: str) -> tuple[str | None, str]:
    """Peel off either a syslog timestamp or RFC3339 timestamp.

    Syslog form: ``Mon DD HH:MM:SS`` (15 chars wide, day may be a
    single digit padded by a space — handled by accepting 14 or 15
    chars).  RFC3339 form: ``YYYY-MM-DDTHH:MM:SS[.fff][±HH:MM|Z]``.
    """
    # RFC3339: starts with 4 digits + '-'
    if len(text) >= 19 and text[:4].isdigit() and text[4] == "-" and text[7] == "-" and text[10] in "T " and text[13] == ":" and text[16] == ":":
        # Find the end of the timestamp (whitespace).
        i = 19
        while i < len(text) and text[i] not in " \t":
            i += 1
        return text[:i], text[i:]
    # Syslog: 3-letter month, space(s), 1-2 digit day, space, HH:MM:SS.
    if len(text) < 14:
        return None, text
    if not text[:3].isalpha():
        return None, text
    parts = text.split(None, 3)
    if len(parts) < 4:
        return None, text
    month, day, hms, rest = parts[0], parts[1], parts[2], parts[3]
    if len(month) != 3 or not day.isdigit() or len(hms) != 8 or hms[2] != ":" or hms[5] != ":":
        return None, text
    return f"{month} {day} {hms}", rest


def _take_token(text: str) -> tuple[str, str]:
    """Return (first whitespace-delimited token, rest)."""
    text = text.lstrip()
    if not text:
        return "", ""
    i = 0
    n = len(text)
    while i < n and not text[i].isspace():
        i += 1
    return text[:i], text[i:]


def _take_severity(text: str) -> tuple[str, str]:
    """Peel a severity word when present; otherwise return ``("", text)``."""
    head, rest = _take_token(text)
    if head.lower() in _F5_SEVERITIES:
        return head.lower(), rest
    return "", text


def _take_daemon_pid(text: str) -> tuple[str, int | None, str]:
    """Peel ``daemon[pid]`` or ``daemon`` off the head.

    The daemon segment ends at either ``:`` (no PID) or ``[`` (PID
    follows).  Returns blank strings when nothing daemon-shaped is
    found so the caller can still recover the body as ``message``.
    """
    text = text.lstrip()
    if not text:
        return "", None, ""
    n = len(text)
    i = 0
    while i < n and text[i] not in "[: \t":
        i += 1
    daemon = text[:i]
    if i < n and text[i] == "[":
        # Optional PID inside the brackets.
        j = i + 1
        while j < n and text[j].isdigit():
            j += 1
        if j > i + 1 and j < n and text[j] == "]":
            try:
                pid = int(text[i + 1 : j])
            except ValueError:
                pid = None
            return daemon, pid, text[j + 1 :]
        return daemon, None, text[i:]
    return daemon, None, text[i:]


def _take_f5_code(text: str) -> tuple[str, str, int | None, str]:
    """Peel an F5 message code (``XXXXXXXX:N``) off the head when present.

    The code is exactly 8 hex digits, a colon, a single digit, and
    a trailing ``:`` followed by a space.  Returns the original
    string under *message* when no code is present so the caller
    sees the raw text.
    """
    if len(text) < 12:
        return "", "", None, text
    if text[8] != ":" or not _is_hex(text[:8]):
        return "", "", None, text
    if not text[9].isdigit():
        return "", "", None, text
    if text[10] != ":":
        return "", "", None, text
    module = text[:8]
    try:
        level = int(text[9])
    except ValueError:
        return "", "", None, text
    code = f"{module}:{level}"
    rest = text[11:].lstrip()
    return code, module, level, rest


def _is_hex(text: str) -> bool:
    """``True`` when every char is a hex digit (upper or lower)."""
    if not text:
        return False
    for ch in text:
        if not (ch.isdigit() or "a" <= ch.lower() <= "f"):
            return False
    return True


def _event_to_dict(event: F5LogEvent) -> dict[str, Any]:
    """Return the event as a plain dict (the shape the DSL navigates).

    Keeping :class:`F5LogEvent` as the in-Python typed form means
    other code (validators, tests) can use the typed handle without
    paying the dict-coercion cost.
    """
    return {
        "timestamp": event.timestamp,
        "host": event.host,
        "severity": event.severity,
        "daemon": event.daemon,
        "pid": event.pid,
        "code": event.code,
        "module": event.module,
        "level": event.level,
        "message": event.message,
        "raw": event.raw,
    }


# ---------------------------------------------------------------------------
# Side-input spec — what the runner remembers about each URI.
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class InputSpec:
    """How to parse a side-input URI.

    *kind* names the format (``"json"`` / ``"jsonl"`` / ``"csv"`` /
    ``"f5log"``); *options* is a small frozen dict of per-format
    knobs.  The most common option today is ``csv_headers`` — a
    tuple of column names supplied via the CLI's
    ``--input-csv NAME=PATH:hdr1,hdr2,…`` form.
    """

    kind: str
    options: tuple[tuple[str, Any], ...] = field(default_factory=tuple)

    def get(self, key: str, default: Any = None) -> Any:
        for k, v in self.options:
            if k == key:
                return v
        return default


# ---------------------------------------------------------------------------
# Errors
# ---------------------------------------------------------------------------


class InputError(Exception):
    """Raised when a side-input file cannot be parsed."""
