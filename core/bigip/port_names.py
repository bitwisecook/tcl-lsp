"""Reverse mapping for BIG-IP port-name → port-number.

BIG-IP's ``mcpd`` ships with a built-in table that maps a fixed set of
service names to L4 port numbers, and ``tmsh`` / SCF output rewrites
numeric ports to that name whenever a match exists.  So an iRule or
virtual-server destination that was configured as ``10.0.0.5:80`` shows
up in the saved configuration as ``10.0.0.5:http``.

The table is **not** ``/etc/services`` — it differs in entries and in
the names BIG-IP picked for some ports (for example ``f5-iquery``,
``f5-globalsite``, and ``any -> 0``).  This module vendors the table
captured from mcpd and exposes a single helper, :func:`resolve_port`,
that turns either a numeric string or a known name into an ``int``.

On output BIG-IP always *accepts* numbers, so we only need the reverse
direction (name → number).  The CSV lives in
``data/scf_port_names.csv`` and is shipped alongside the package.
"""

from __future__ import annotations

import csv
from functools import lru_cache
from pathlib import Path

_DATA_PATH = Path(__file__).with_name("data") / "scf_port_names.csv"


@lru_cache(maxsize=1)
def _name_to_port() -> dict[str, int]:
    table: dict[str, int] = {}
    with _DATA_PATH.open(encoding="utf-8", newline="") as fh:
        for row in csv.reader(fh):
            if len(row) != 2:
                continue
            name, num = row[0].strip(), row[1].strip()
            if not name or not num.isdigit():
                continue
            table[name.lower()] = int(num)
    return table


def port_name_to_number(name: str) -> int | None:
    """Return the BIG-IP port number for *name*, or ``None`` if unknown.

    Lookup is case-insensitive — BIG-IP itself emits names in lower
    case, but accepting any case keeps the helper forgiving when fed
    hand-edited configuration.
    """
    if not name:
        return None
    return _name_to_port().get(name.lower())


def resolve_port(value: str) -> int | None:
    """Resolve a port token to an integer.

    Accepts either a decimal string (``"443"``) or a BIG-IP service
    name (``"https"``).  Returns ``None`` if *value* is neither a
    valid port number in ``0..65535`` nor a known name — callers
    typically treat that as "leave the field as-is".
    """
    if not value:
        return None
    if value.isdigit():
        num = int(value)
        if 0 <= num <= 65535:
            return num
        return None
    return port_name_to_number(value)
