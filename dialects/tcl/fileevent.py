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

# Scaffolded from fileevent.n -- refine and commit
"""fileevent -- Execute a script when a channel becomes readable or writable."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.dialects import DIALECTS_EXCEPT_IRULES
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
)
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page fileevent.n"


@register
class FileeventCommand(CommandDef):
    name = "fileevent"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="fileevent",
            byte_compiled=True,
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Execute a script when a channel becomes readable or writable",
                synopsis=(
                    "fileevent channel readable ?script?",
                    "fileevent channel writable ?script?",
                ),
                snippet="The fileevent command has been superseded by the chan event command which supports the same syntax and options.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="fileevent channel readable ?script?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2, 3),
            ),
            arg_roles={0: frozenset({ArgRole.CHANNEL}), 2: frozenset({ArgRole.BODY})},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
