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

# Scaffolded from unset.n -- refine and commit
"""unset -- Delete variables."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page unset.n"


@register
class UnsetCommand(CommandDef):
    name = "unset"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="unset",
            byte_compiled=True,
            frameless_runtime=True,
            hover=HoverSnippet(
                summary="Delete variables",
                synopsis=("unset ?-nocomplain? ?--? ?name name name ...?",),
                snippet="This command removes one or more variables.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="unset ?-nocomplain? ?--? ?name name name ...?",
                    options=(
                        OptionSpec(
                            name="-nocomplain", detail="Suppress errors for non-existent variables."
                        ),
                        OptionSpec(name="--", detail="End of options."),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            assigns_variable_at=0,
            destroys_variable=True,
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
