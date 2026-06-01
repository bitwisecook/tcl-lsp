"""Importer for F5 BIG-IP ``rule-profiler`` occurrence logs.

The rule-profiler writes one CSV line per occurrence to its configured
syslog publisher (e.g. ``/var/log/ltm``).  Each line has the shape::

    <ts>,RP_<TYPE>,<vs>,<value>,<pid>,<flow>,<r_ip>,<r_port>,<r_rd>,<l_ip>,<l_port>,<l_rd>[,<depth>]

for example::

    1511494952932589,RP_EVENT_ENTRY,/Common/vs1,CLIENT_ACCEPTED,18291,0x9455...,10.10.10.2,46052,0,10.10.10.160,80,
    1511494952932633,RP_CMD_EXIT,/Common/vs1,IP::remote_addr,18291,0x9455...,10.10.10.2,46052,0,10.10.10.160,80,0

This module translates such a log into the normalised
:mod:`~compiler.pgo.occurrence` stream and rolls it up into a
:class:`~compiler.pgo.profile_data.ProfileData`.

**Limitation:** F5 logs carry command / event *names* but no source line
numbers, so this importer yields ``command_counts`` / ``event_counts``
(coarse) — never ``line_counts``.  Branch attribution from an F5 log is
therefore best-effort (by the first command name in each branch body).
For precise per-line attribution use :mod:`compiler.pgo.tclsh_capture`.
"""

from __future__ import annotations

from .occurrence import RP_OCCURRENCE_KINDS, FlowContext, Occurrence
from .profile_data import ProfileData


def _maybe_int(text: str, *, base: int = 10) -> int | None:
    text = text.strip()
    if not text:
        return None
    try:
        return int(text, base)
    except ValueError:
        return None


def _field(fields: list[str], idx: int) -> str:
    return fields[idx].strip() if idx < len(fields) else ""


def parse_f5_occurrences(text: str) -> list[Occurrence]:
    """Parse rule-profiler log *text* into a list of :class:`Occurrence`.

    Lines that are blank, malformed, or carry an unrecognised occurrence
    type are skipped, so partial / truncated captures parse cleanly.
    """
    occurrences: list[Occurrence] = []
    for raw in text.splitlines():
        line = raw.strip()
        if not line:
            continue
        fields = line.split(",")
        if len(fields) < 4:
            continue
        kind = RP_OCCURRENCE_KINDS.get(_field(fields, 1))
        if kind is None:
            continue  # comment line, header, or unknown occurrence type
        # Flow id is logged as hex (``0x...``); keep the raw token as the
        # flow key and parse pid/ports/rd as best-effort integers.
        context = FlowContext(
            virtual_server=_field(fields, 2) or None,
            process_id=_maybe_int(_field(fields, 4)),
            flow_id=_field(fields, 5) or None,
            remote_addr=_field(fields, 6) or None,
            remote_port=_maybe_int(_field(fields, 7)),
            remote_rd=_maybe_int(_field(fields, 8)),
            local_addr=_field(fields, 9) or None,
            local_port=_maybe_int(_field(fields, 10)),
            local_rd=_maybe_int(_field(fields, 11)),
        )
        occurrences.append(
            Occurrence(
                kind=kind,
                value=_field(fields, 3),
                timestamp_us=_maybe_int(_field(fields, 0)),
                line=None,  # F5 logs have no source line
                depth=_maybe_int(_field(fields, 12)),
                context=context,
            )
        )
    return occurrences


def parse_f5_log(text: str) -> ProfileData:
    """Parse rule-profiler log *text* into aggregated :class:`ProfileData`."""
    return ProfileData.from_occurrences(parse_f5_occurrences(text), source="f5-log")
