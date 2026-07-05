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

# Scaffolded from update.n -- refine and commit
"""update -- Process pending events and idle callbacks."""

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
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page update.n"


@register
class UpdateCommand(CommandDef):
    name = "update"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="update",
            byte_compiled=True,
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Process pending events and idle callbacks",
                synopsis=("update ?idletasks?",),
                snippet="This command is used to bring the application by entering the event loop repeatedly until all pending events (including idle callbacks) have been processed.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="update ?idletasks?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0, 1),
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
