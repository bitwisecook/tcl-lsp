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

"""lmap -- Iterate over all elements in one or more lists and collect results."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register
from .shimmer_resolvers import resolve_lmap

_SOURCE = "Tcl man page lmap.n"


def _lmap_arg_roles(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """Last argument is the body script."""
    if len(args) >= 3:
        return {len(args) - 1: frozenset({ArgRole.BODY})}
    return {}


@register
class LmapCommand(CommandDef):
    name = "lmap"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lmap",
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            loop_list_header=True,
            hover=HoverSnippet(
                summary="Iterate over all elements in one or more lists and collect results",
                synopsis=(
                    "lmap varname list body",
                    "lmap varlist1 list1 ?varlist2 list2 ...? body",
                ),
                snippet="The lmap command implements a loop where the loop variable(s) take on values from one or more lists, and the loop returns a list of results collected from each iteration.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lmap varname list body",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(3),
            ),
            arg_role_resolver=_lmap_arg_roles,
            return_type=TclType.LIST,
            arg_type_resolver=resolve_lmap,
            side_effect_hints=(
                SideEffect(target=SideEffectTarget.UNKNOWN, connection_side=ConnectionSide.NONE),
            ),
        )
