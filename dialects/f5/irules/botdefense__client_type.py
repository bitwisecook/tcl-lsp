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
"""BOTDEFENSE::client_type -- Returns the client type: browser, mobile application or bot."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__client_type.html"


@register
class BotdefenseClientTypeCommand(CommandDef):
    name = "BOTDEFENSE::client_type"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::client_type",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the client type: browser, mobile application or bot.",
                synopsis=("BOTDEFENSE::client_type",),
                snippet=(
                    "Returns the client type. The returned value is one of the following strings:\n"
                    "    * bot - if the client was detected as a bot.\n"
                    "    * mobile_app - if the client is a mobile app using F5 Anti Bot mobile SDK.\n"
                    "    * browser - if the client is a Web browser.\n"
                    "    * uncategorized - if the client type could not be determined."
                ),
                source=_SOURCE,
                examples=(
                    "EXAMPLE: Redirect bots to a honeypot page\n"
                    " when BOTDEFENSE_ACTION {\n"
                    '     if {[BOTDEFENSE::client_type] eq "bot"} {\n'
                    '         set log "Request from a Bot on "\n'
                    '         append log "IP [IP::client_addr]"\n'
                    "         HSL::send $hsl $log\n"
                    '         HTTP::redirect "https://www.example.com/honeypot.html"\n'
                    "      }\n"
                    " }"
                ),
                return_value="A string signifying the client type.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::client_type",
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
