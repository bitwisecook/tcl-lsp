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

# Scaffolded from variable.n -- refine and commit
"""variable -- create and initialize a namespace variable."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page variable.n"


@register
class VariableCommand(CommandDef):
    name = "variable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="variable",
            byte_compiled=True,
            frameless_runtime=True,
            not_proc_factory=True,
            is_language_keyword=True,
            hover=HoverSnippet(
                summary="create and initialize a namespace variable",
                synopsis=(
                    "variable name",
                    "variable ?name value...?",
                ),
                snippet="This command is normally used within a namespace eval command to create one or more variables within a namespace.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="variable name",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            creates_scope_alias=True,
            creates_dynamic_barrier=True,
            assigns_variable_at=0,
            arg_roles={0: frozenset({ArgRole.VAR_WRITE})},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
