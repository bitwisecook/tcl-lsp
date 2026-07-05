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

"""for -- C-style loop with init, test, and next scripts."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.registry.type_hints import ArgTypeHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl for(1)"


@register
class ForCommand(CommandDef):
    name = "for"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="for",
            byte_compiled=True,
            not_proc_factory=True,
            is_control_flow=True,
            is_language_keyword=True,
            has_boolean_condition=True,
            never_inline_body=True,
            has_loop_body=True,
            hover=HoverSnippet(
                summary="C-style loop with init, test, and next scripts.",
                synopsis=("for start test next body",),
                snippet="`start` runs once; loop continues while `test` is true; `next` runs after each body pass.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="for start test next body",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(4, 4),
            ),
            arg_roles={
                0: frozenset({ArgRole.BODY}),
                1: frozenset({ArgRole.EXPR}),
                2: frozenset({ArgRole.BODY}),
                3: frozenset({ArgRole.BODY}),
            },
            return_type=TclType.STRING,
            arg_types={1: ArgTypeHint(expected=TclType.BOOLEAN, shimmers=True)},
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
