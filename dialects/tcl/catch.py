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

# Scaffolded from catch.n -- refine and commit
"""catch -- Evaluate script and trap exceptional returns."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page catch.n"


@register
class CatchCommand(CommandDef):
    name = "catch"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="catch",
            byte_compiled=True,
            not_proc_factory=True,
            is_control_flow=True,
            is_language_keyword=True,
            hover=HoverSnippet(
                summary="Evaluate script and trap exceptional returns",
                synopsis=("catch script ?resultVarName? ?optionsVarName?",),
                snippet="The catch command may be used to prevent errors from aborting command interpretation.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="catch script ?resultVarName? ?optionsVarName?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 3),
            ),
            arg_roles={
                0: frozenset({ArgRole.BODY}),
                1: frozenset({ArgRole.VAR_WRITE}),
                2: frozenset({ArgRole.VAR_WRITE}),
            },
            return_type=TclType.INT,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
