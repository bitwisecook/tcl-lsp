"""F5 BIG-IP configuration parser (package facade).

Re-exports :func:`parse_bigip_conf` — the only documented public
function.  The full implementation (~5800 lines of header
parsing, per-kind sub-parsers, dispatch tables, and the
top-level driver) lives in :mod:`._impl`.  Tests that touch
internals reach for them as ``core.bigip.parser._impl.<name>``
or via the legacy alias ``core.bigip.parser.<name>`` which still
resolves through ``__getattr__`` below.

Splitting the per-module parsers into individual submodules
(``_ltm.py`` / ``_net.py`` / …) is a follow-up; this package
layout sets up that refactor without breaking any of the
500+ existing call sites.
"""

from __future__ import annotations

from typing import Any

from ._impl import parse_bigip_conf

__all__ = ["parse_bigip_conf"]


def __getattr__(name: str) -> Any:
    """Backwards-compat shim — internal parser helpers (``_parse_*``
    / ``_extract_blocks`` / ``_Block`` / ``_parse_generic_header``,
    etc.) used to live at ``core.bigip.parser.<name>``.  After the
    package conversion they're at ``core.bigip.parser._impl.<name>``
    — this hook keeps the old import paths working without forcing
    every test and tool to update its imports.
    """
    from . import _impl

    try:
        return getattr(_impl, name)
    except AttributeError as exc:
        raise AttributeError(f"module 'core.bigip.parser' has no attribute {name!r}") from exc
