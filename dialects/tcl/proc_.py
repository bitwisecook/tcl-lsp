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

"""proc -- Create a Tcl procedure."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity, BodyKind
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl proc(1)"


@register
class ProcCommand(CommandDef):
    name = "proc"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="proc",
            byte_compiled=True,
            not_proc_factory=True,
            is_language_keyword=True,
            never_inline_body=True,
            hover=HoverSnippet(
                summary="Create a Tcl procedure.",
                synopsis=("proc name args body",),
                snippet="`args` is a formal parameter list; `body` executes in a new call frame.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="proc name args body",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(3, 3),
            ),
            wasm_emits_nothing=True,
            arg_roles={
                0: frozenset({ArgRole.NAME}),
                1: frozenset({ArgRole.PARAM_LIST}),
                2: frozenset({ArgRole.BODY}),
            },
            body_kind=BodyKind.STRUCTURAL,
            return_type=TclType.STRING,
            defines_procedure=True,
            irules_top_level_only=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.PROC_DEFINITION,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
