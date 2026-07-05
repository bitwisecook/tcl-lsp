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

# Scaffolded from pid.n -- refine and commit
"""pid -- Retrieve process identifiers."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.dialects import DIALECTS_EXCEPT_IRULES
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page pid.n"


@register
class PidCommand(CommandDef):
    name = "pid"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="pid",
            byte_compiled=True,
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Retrieve process identifiers",
                synopsis=("pid ?fileId?",),
                snippet="If the fileId argument is given then it should normally refer to a process pipeline created with the open command.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="pid ?fileId?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0, 1),
            ),
            return_type=TclType.INT,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
