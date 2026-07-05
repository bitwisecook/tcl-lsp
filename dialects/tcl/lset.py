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

# Scaffolded from lset.n -- refine and commit
"""lset -- Change an element in a list."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.registry.type_hints import ArgTypeHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageType
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page lset.n"


@register
class LsetCommand(CommandDef):
    name = "lset"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lset",
            hover=HoverSnippet(
                summary="Change an element in a list",
                synopsis=("lset varName ?index ...? newValue",),
                snippet="The lset command accepts a parameter, varName, which it interprets as the name of a variable containing a Tcl list.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lset varName ?index ...? newValue",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            assigns_variable_at=0,
            arg_roles={0: frozenset({ArgRole.VAR_WRITE})},
            return_type=TclType.LIST,
            inferred_storage_type=StorageType.LIST,
            arg_types={0: ArgTypeHint(expected=TclType.LIST, shimmers=True)},
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
