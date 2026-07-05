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

"""expr -- Evaluate a Tcl expression."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.registry.type_hints import ArgTypeHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl expr(1)"


@register
class ExprCommand(CommandDef):
    name = "expr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="expr",
            byte_compiled=True,
            frameless_runtime=True,
            not_proc_factory=True,
            needs_start_cmd=True,
            hover=HoverSnippet(
                summary="Evaluate a Tcl expression.",
                synopsis=("expr arg ?arg ...?",),
                snippet=(
                    "**Always brace expressions**: `expr {$a + $b}`.\n\n"
                    "Without braces, `expr $x + 1` undergoes double "
                    "substitution: the Tcl parser expands `$x` first, then "
                    "`expr` evaluates the result. If `$x` contains "
                    "`[dangerous_command]`, it executes. Bracing also enables "
                    "bytecode compilation for better performance."
                ),
                source=_SOURCE,
                return_value="The result of evaluating the expression.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="expr arg ?arg ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            taint_sink=True,
            arg_roles={0: frozenset({ArgRole.EXPR})},
            return_type=TclType.NUMERIC,
            arg_types={0: ArgTypeHint(expected=TclType.NUMERIC, shimmers=True)},
            pure_evaluation=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
