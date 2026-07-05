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

"""The ``encoding`` command — stub for init.tcl support."""

from __future__ import annotations

from typing import TYPE_CHECKING

from ..types import TclError, TclResult

if TYPE_CHECKING:
    from ..interp import TclInterp


def _cmd_encoding(interp: TclInterp, args: list[str]) -> TclResult:
    """encoding subcommand ?arg ...?"""
    if not args:
        raise TclError('wrong # args: should be "encoding subcommand ?arg ...?"')

    match args[0]:
        case "dirs":
            if len(args) > 1:
                interp._encoding_dirs = args[1]  # type: ignore[attr-defined]
                return TclResult()
            return TclResult(value=getattr(interp, "_encoding_dirs", ""))
        case "system":
            if len(args) > 1:
                return TclResult()  # ignore set, always utf-8
            return TclResult(value="utf-8")
        case "convertfrom":
            # encoding convertfrom ?encoding? data
            return TclResult(value=args[-1])
        case "convertto":
            # encoding convertto ?encoding? data
            return TclResult(value=args[-1])
        case "names":
            return TclResult(value="utf-8 ascii iso8859-1 unicode")
        case _:
            raise TclError(
                f'unknown or ambiguous subcommand "{args[0]}": must be '
                "convertfrom, convertto, dirs, names, or system"
            )


def register() -> None:
    """Register encoding command."""
    from compiler.registry import REGISTRY

    REGISTRY.register_handler("encoding", _cmd_encoding)
