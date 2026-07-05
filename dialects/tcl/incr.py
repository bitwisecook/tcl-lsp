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

# Scaffolded from incr.n -- refine and commit
"""incr -- Increment the value of a variable."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.dialects import dialects_since
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.registry.type_hints import ArgTypeHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page incr.n"


@register
class IncrCommand(CommandDef):
    name = "incr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="incr",
            byte_compiled=True,
            frameless_runtime=True,
            hover=HoverSnippet(
                summary="Increment the value of a variable",
                synopsis=("incr varName ?increment?",),
                snippet="Increments the value stored in the variable whose name is varName.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="incr varName ?increment?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 2),
            ),
            assigns_variable_at=0,
            reads_variable_before_write=True,
            # incr safely treats an uninitialised variable as 0 in Tcl 8.5+.
            # In 8.4 (and dialects based on it, e.g. iRules) it raises
            # "can't read": no such variable.
            safe_on_uninit=dialects_since("tcl8.5"),
            arg_roles={0: frozenset({ArgRole.VAR_WRITE})},
            return_type=TclType.INT,
            arg_types={
                0: ArgTypeHint(expected=TclType.INT, shimmers=True),
                1: ArgTypeHint(expected=TclType.INT, shimmers=True),
            },
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
