"""F5 BIG-IP configuration parser (package facade).

Re-exports :func:`parse_bigip_conf` — the only documented public
entry point.  Implementation split:

- :mod:`._helpers` — brace-balanced block extraction, property
  / list-block parsing, header-tuple parsing.  Pure utilities,
  no model dependencies.
- :mod:`._parsers` — every per-kind ``_parse_*`` function plus
  the dispatch tables that map kind labels to those parsers.
- :mod:`._driver` — :func:`parse_bigip_conf` itself, reading the
  dispatch tables from :mod:`._parsers` to route each block to
  the right sub-parser.

Internal helpers (``_extract_blocks`` / ``_parse_*`` / ``_Block``
/ etc.) are still reachable via ``core.bigip.parser._impl`` for
backwards compatibility — wait, no, that file is gone.  Callers
that need internals should reach into the specific submodule
(``core.bigip.parser._helpers`` for block/property helpers,
``core.bigip.parser._parsers`` for per-kind functions).
"""

from __future__ import annotations

from ._driver import parse_bigip_conf

__all__ = ["parse_bigip_conf"]
