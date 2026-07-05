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

# Enriched from F5 iRules reference documentation.
"""LSN::disable -- Disables LSN translation for the current connection if LSN translation has been configured."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, _LSN_EVENT_REQUIRES, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LSN__disable.html"


@register
class LsnDisableCommand(CommandDef):
    name = "LSN::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LSN::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disables LSN translation for the current connection if LSN translation has been configured.",
                synopsis=("LSN::disable",),
                snippet=(
                    "Disables LSN translation for the current connection if LSN translation has been configured.\n"
                    "\n"
                    "Arguments:\n"
                    "    LSN::disable - If LSN translation is configured, disables translation for this connection."
                ),
                source=_SOURCE,
                examples=("when HTTP_REQUEST {\n    LSN::disable\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LSN::disable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=_LSN_EVENT_REQUIRES,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LSN_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
