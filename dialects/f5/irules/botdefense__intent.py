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
"""BOTDEFENSE::intent -- Returns the intent found for the bot that sent the current request."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__intent.html"


@register
class BotdefenseIntentCommand(CommandDef):
    name = "BOTDEFENSE::intent"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::intent",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the intent found for the bot that sent the current request.",
                synopsis=("BOTDEFENSE::intent",),
                snippet="Returns the intent found for the bot that sent the current request. The intent is based on the micro-service anomaly found for that client and may have been detected in a previous request of the client, not necessarily the present request",
                source=_SOURCE,
                examples=(
                    "when BOTDEFENSE_ACTION {\n"
                    '    if {[BOTDEFENSE::intent] contains "OAT"} {\n'
                    "        BOTDEFENSE::action block\n"
                    "    }\n"
                    "}"
                ),
                return_value="Returns the intent found for the bot that sent the current request based on a micro-service anomaly found for that bot, or empty string if no intent was found. The possible intents are those available per the various micro-services types.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::intent",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"BOTDEFENSE"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
