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

"""Arg-type resolvers for variable-layout Tcl commands.

These functions are called by the shimmer detector when a command's
argument layout depends on the invocation (options shift positions,
variadic args, alternating patterns).  Each resolver receives the
actual argument tuple and returns a dict mapping position indices
to ``ArgTypeHint`` instances.
"""

from __future__ import annotations

from compiler.registry.type_hints import ArgTypeHint
from compiler.types import TclType

_LIST = ArgTypeHint(expected=TclType.LIST, shimmers=True)
_DICT = ArgTypeHint(expected=TclType.DICT, shimmers=True)


def resolve_lsort(args: tuple[str, ...]) -> dict[int, ArgTypeHint]:
    """``lsort ?options...? list`` — last arg is the list."""
    if not args:
        return {}
    return {len(args) - 1: _LIST}


def resolve_lsearch(args: tuple[str, ...]) -> dict[int, ArgTypeHint]:
    """``lsearch ?options...? list pattern`` — second-to-last is the list."""
    if len(args) < 2:
        return {}
    return {len(args) - 2: _LIST}


def resolve_foreach(args: tuple[str, ...]) -> dict[int, ArgTypeHint]:
    """``foreach varList list ?varList list...? body`` — odd positions are lists.

    Layout: args[0]=varList, args[1]=list, args[2]=varList, args[3]=list,
    ..., args[-1]=body.  List arguments are at odd indices up to len-2.
    """
    if len(args) < 3:
        return {}
    return {i: _LIST for i in range(1, len(args) - 1, 2)}


resolve_lmap = resolve_foreach


def resolve_concat(args: tuple[str, ...]) -> dict[int, ArgTypeHint]:
    """``concat ?arg...?`` — all args shimmer to list."""
    return {i: _LIST for i in range(len(args))}


def resolve_dict_merge(args: tuple[str, ...]) -> dict[int, ArgTypeHint]:
    """``dict merge ?dictValue...?`` — all sub-args shimmer to dict.

    Receives args after the ``merge`` subcommand word.  Indices are
    0-based in the SubCommand arg space.
    """
    return {i: _DICT for i in range(len(args))}
