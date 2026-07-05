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
"""TCP::respond -- Sends the specified data directly to the peer."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__respond.html"


@register
class TcpRespondCommand(CommandDef):
    name = "TCP::respond"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::respond",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sends the specified data directly to the peer.",
                synopsis=("TCP::respond RESPONSE_STRING",),
                snippet=(
                    "Sends the specified data directly to the peer. This command can be used\n"
                    "to complete a protocol handshake via an iRule."
                ),
                source=_SOURCE,
                examples=("when SERVER_CONNECTED {\n    peer { TCP::collect 4 }\n}"),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::respond RESPONSE_STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                transport="tcp",
                also_in=frozenset({"SIP_REQUEST", "SIP_REQUEST_SEND", "SIP_RESPONSE"}),
            ),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
