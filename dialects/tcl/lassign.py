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

"""lassign -- Assign list elements to variables."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.registry.type_hints import ArgTypeHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page lassign.n"


def _lassign_arg_roles(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """D4-F2 closure: ``lassign list ?varName ...?`` -- every arg after
    the list (index >= 1) is a varName.  Dynamic resolver supports any
    number of trailing varName args (the previous hard-coded 18 slots
    falsely flagged W210 on lassigns with more)."""
    return {i: frozenset({ArgRole.VAR_WRITE}) for i in range(1, len(args))}


@register
class LassignCommand(CommandDef):
    name = "lassign"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lassign",
            frameless_runtime=True,
            dialects=frozenset({"tcl8.5", "tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="Assign list elements to variables",
                synopsis=("lassign list ?varName ...?",),
                snippet="This command treats the value list as a list and assigns successive elements from that list to the variables given by the varName arguments in order.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lassign list ?varName ...?",
                ),
            ),
            arg_role_resolver=_lassign_arg_roles,
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            return_type=TclType.LIST,
            arg_types={0: ArgTypeHint(expected=TclType.LIST, shimmers=True)},
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
