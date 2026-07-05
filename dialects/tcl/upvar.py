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

# Scaffolded from upvar.n -- refine and commit
"""upvar -- Create link to variable in a different stack frame."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page upvar.n"


@register
class UpvarCommand(CommandDef):
    name = "upvar"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="upvar",
            byte_compiled=True,
            frameless_runtime=True,
            not_proc_factory=True,
            is_language_keyword=True,
            creates_dynamic_barrier=True,
            hover=HoverSnippet(
                summary="Create link to variable in a different stack frame",
                synopsis=("upvar ?level? otherVar myVar ?otherVar myVar ...?",),
                snippet="This command arranges for one or more local variables in the current procedure to refer to variables in an enclosing procedure call or to global variables.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="upvar ?level? otherVar myVar ?otherVar myVar ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            creates_scope_alias=True,
            xc_translatable=False,
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
