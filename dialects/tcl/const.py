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

"""const -- Define a constant variable (Tcl 9 / TIP 590)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl const(1)"


@register
class ConstCommand(CommandDef):
    name = "const"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="const",
            hover=HoverSnippet(
                summary="Define a constant variable.",
                synopsis=("const varName value",),
                snippet=(
                    "Stores ``value`` in ``varName`` and marks the variable as constant: "
                    "further attempts to ``set`` or ``unset`` it raise an error. "
                    "Re-applying ``const`` to an existing constant of the same name is a "
                    "silent no-op; applying it to an existing non-constant variable raises."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="const varName value",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2, 2),
            ),
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
