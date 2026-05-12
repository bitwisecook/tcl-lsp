"""Reverse mapping for BIG-IP port-name → port-number.

BIG-IP's ``mcpd`` ships with a built-in table that maps a fixed set of
service names to L4 port numbers, and ``tmsh`` / SCF output rewrites
numeric ports to that name whenever a match exists.  So an iRule or
virtual-server destination configured as ``10.0.0.5:443`` shows up in
the saved configuration as ``10.0.0.5:https``.

The table is **not** ``/etc/services`` — it differs in entries and in
the names BIG-IP picked for some ports (for example ``f5-iquery``,
``f5-globalsite``, and ``any -> 0``; conversely there's no entry for
``http``/80).  The canonical source lives in
``core/bigip/data/scf_port_names.csv`` and is codegen'd into
:mod:`core.bigip._port_names_table` so the dict literal can be
unmarshalled straight from the zipapp bytecode — no archive read, no
csv parse on cold start.  The reverse lookup is a single dict
comprehension at import time.

On output BIG-IP always *accepts* numbers, so callers mainly want the
name → number direction via :func:`resolve_port`.
"""

from __future__ import annotations

from ._port_names_table import NAME_TO_PORT

# Two-way: forward is bundled, reverse is built once at import.  The
# table has no duplicate ports today, so this is a true inverse — if
# the CSV ever introduces a collision the codegen check will catch it.
PORT_TO_NAME: dict[int, str] = {port: name for name, port in NAME_TO_PORT.items()}


def port_name_to_number(name: str) -> int | None:
    """Return the BIG-IP port number for *name*, or ``None`` if unknown.

    Lookup is case-insensitive — BIG-IP itself emits names in lower
    case, but accepting any case keeps the helper forgiving when fed
    hand-edited configuration.
    """
    if not name:
        return None
    return NAME_TO_PORT.get(name.lower())


def port_number_to_name(port: int) -> str | None:
    """Return the BIG-IP service name for *port*, or ``None`` if unnamed.

    Provided for symmetry / display.  Most callers only need the
    forward direction since BIG-IP accepts numeric ports on input.
    """
    return PORT_TO_NAME.get(port)


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
