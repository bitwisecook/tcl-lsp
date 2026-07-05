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

# Scaffolded from after.n -- refine and commit
"""after -- Execute a command after a time delay."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
)
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page after.n"


_av = make_av(_SOURCE)


@register
class AfterCommand(CommandDef):
    name = "after"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="after",
            byte_compiled=True,
            hover=HoverSnippet(
                summary="Execute a command after a time delay",
                synopsis=(
                    "after ms",
                    "after ms ?script script script ...?",
                    "after cancel id",
                    "after cancel script script script ...",
                ),
                snippet="This command is used to delay execution of the program or to execute a command in background sometime in the future.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="after ms",
                    arg_values={
                        0: (
                            _av(
                                "cancel",
                                "Cancels the execution of a delayed command that was previously scheduled.",
                                "after cancel id",
                            ),
                            _av(
                                "idle",
                                "Concatenates the script arguments together with space separators (just as in the concat command), and arranges for the resulting script to be evaluated later as an idle callback.",
                                "after idle script ?script script ...?",
                            ),
                            _av(
                                "info",
                                "This command returns information about existing event handlers.",
                                "after info ?id?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
