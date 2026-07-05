# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

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
/ etc.) are still reachable via ``dialects.f5.bigip.parser._impl`` for
backwards compatibility — wait, no, that file is gone.  Callers
that need internals should reach into the specific submodule
(``dialects.f5.bigip.parser._helpers`` for block/property helpers,
``dialects.f5.bigip.parser._parsers`` for per-kind functions).
"""

from __future__ import annotations

from ._driver import parse_bigip_conf

__all__ = ["parse_bigip_conf"]
