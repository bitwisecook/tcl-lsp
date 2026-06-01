"""Importer for F5 BIG-IP ``rule-profiler`` occurrence logs.

The rule-profiler writes one CSV record per occurrence to its configured
syslog publisher.  On a typical ``/var/log/ltm`` destination each record is
embedded in a syslog line, e.g.::

    Nov 24 06:22:32 bigip1 info tmm[18291]: 1511494952932589,RP_EVENT_ENTRY,/Common/vs1,CLIENT_ACCEPTED,18291,0x9455,10.10.10.2,46052,0,10.10.10.160,80,

The CSV payload (everything from the timestamp onward) has the field order::

    <ts>,RP_<TYPE>,<vs>,<value>,<pid>,<flow>,<r_ip>,<r_port>,<r_rd>,<l_ip>,<l_port>,<l_rd>[,<depth>]

This importer is tolerant of an arbitrary syslog prefix: it anchors on the
``<digits>,RP_<TYPE>,`` payload start, so both raw ``/var/log/ltm`` lines
and already-stripped CSV (as in the DevCentral articles) parse cleanly.
The field layout matches the community ``bigip-irule-profiler`` parser
(mhermsdorferf5/bigip-irule-profiler).

It translates the log into the normalised :mod:`~compiler.pgo.occurrence`
stream and rolls it up into a :class:`~compiler.pgo.profile_data.ProfileData`.

**Limitation:** F5 logs carry command / event *names* but no source line
numbers, so this importer yields ``command_counts`` / ``event_counts``
(coarse) — never ``line_counts``.  Branch attribution from an F5 log is
therefore best-effort (by the first command name in each branch body).
For precise per-line attribution use :mod:`compiler.pgo.tclsh_capture`.
"""

from __future__ import annotations

import re

from .occurrence import RP_OCCURRENCE_KINDS, FlowContext, Occurrence
from .profile_data import ProfileData

#: Anchor on the ``<timestamp>,RP_<TYPE>,<rest…>`` payload, ignoring any
#: leading syslog prefix.  ``RP_`` cannot appear in a syslog prefix in this
#: shape (a pid like ``tmm[18291]:`` is not followed by ``,RP_``), so the
#: first match is always the real record.
_RP_LINE = re.compile(r"(\d+),(RP_[A-Z_]+),(.*)$")


def _maybe_int(text: str, *, base: int = 10) -> int | None:
    text = text.strip()
    if not text:
        return None
    try:
        return int(text, base)
    except ValueError:
        return None


def parse_f5_occurrences(text: str) -> list[Occurrence]:
    """Parse rule-profiler log *text* into a list of :class:`Occurrence`.

    Lines that are blank, malformed, or carry an unrecognised occurrence
    type are skipped, so partial / truncated captures parse cleanly.
    """
    occurrences: list[Occurrence] = []
    for raw in text.splitlines():
        match = _RP_LINE.search(raw)
        if match is None:
            continue
        kind = RP_OCCURRENCE_KINDS.get(match.group(2))
        if kind is None:
            continue  # unknown occurrence type
        fields = [f.strip() for f in match.group(3).split(",")]

        def field(idx: int, _fields: list[str] = fields) -> str:
            return _fields[idx] if idx < len(_fields) else ""

        context = FlowContext(
            virtual_server=field(0) or None,
            process_id=_maybe_int(field(2)),
            flow_id=field(3) or None,
            remote_addr=field(4) or None,
            remote_port=_maybe_int(field(5)),
            remote_rd=_maybe_int(field(6)),
            local_addr=field(7) or None,
            local_port=_maybe_int(field(8)),
            local_rd=_maybe_int(field(9)),
        )
        occurrences.append(
            Occurrence(
                kind=kind,
                value=field(1),
                timestamp_us=_maybe_int(match.group(1)),
                line=None,  # F5 logs have no source line
                depth=_maybe_int(field(10)),
                context=context,
            )
        )
    return occurrences


def parse_f5_log(text: str) -> ProfileData:
    """Parse rule-profiler log *text* into aggregated :class:`ProfileData`."""
    return ProfileData.from_occurrences(parse_f5_occurrences(text), source="f5-log")
