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

# Scaffolded from while.n -- refine and commit
"""while -- Execute script repeatedly as long as a condition is met."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.registry.type_hints import ArgTypeHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page while.n"


@register
class WhileCommand(CommandDef):
    name = "while"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="while",
            byte_compiled=True,
            not_proc_factory=True,
            is_control_flow=True,
            is_language_keyword=True,
            has_boolean_condition=True,
            has_loop_body=True,
            never_inline_body=True,
            hover=HoverSnippet(
                summary="Execute script repeatedly as long as a condition is met",
                synopsis=("while test body",),
                snippet="The while command evaluates test as an expression (in the same way that expr evaluates its argument).",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="while test body",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2, 2),
            ),
            arg_roles={0: frozenset({ArgRole.EXPR}), 1: frozenset({ArgRole.BODY})},
            return_type=TclType.STRING,
            arg_types={0: ArgTypeHint(expected=TclType.BOOLEAN, shimmers=True)},
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
