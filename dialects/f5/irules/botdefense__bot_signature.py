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
"""BOTDEFENSE::bot_signature -- Returns the name of the detected Bot Signature."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__bot_signature.html"


@register
class BotdefenseBotSignatureCommand(CommandDef):
    name = "BOTDEFENSE::bot_signature"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::bot_signature",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the name of the detected Bot Signature.",
                synopsis=("BOTDEFENSE::bot_signature",),
                snippet="Returns the name of the detected Bot Signature, or an empty string if no bot signature was detected.",
                source=_SOURCE,
                examples=(
                    "# EXAMPLE: Log the bot signature.\n"
                    "when BOTDEFENSE_REQUEST {\n"
                    '    set log "botdefense bot_signature is"\n'
                    '    append log " [BOTDEFENSE::bot_signature]"\n'
                    "    HSL::send $hsl $log\n"
                    "}"
                ),
                return_value="Returns the name of the detected Bot Signature, or an empty string if no bot signature was detected.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::bot_signature",
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
