"""F5 BIG-IP configuration parser (package facade).

Re-exports :func:`parse_bigip_conf` — the only documented public
function.  The full implementation (~5800 lines of header
parsing, per-kind sub-parsers, dispatch tables, and the
top-level driver) lives in :mod:`._impl`.

Splitting the per-module parsers into individual submodules
(``_ltm.py`` / ``_net.py`` / …) is a follow-up; this package
layout sets up that refactor without breaking any of the
500+ existing call sites, all of which already use
``from core.bigip.parser import parse_bigip_conf``.
"""

from __future__ import annotations

from ._impl import parse_bigip_conf

__all__ = ["parse_bigip_conf"]
