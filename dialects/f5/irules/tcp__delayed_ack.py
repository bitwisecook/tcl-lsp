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
"""TCP::delayed_ack -- Toggles TCP delayed acknowledgements (ACKs)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__delayed_ack.html"


@register
class TcpDelayedAckCommand(CommandDef):
    name = "TCP::delayed_ack"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::delayed_ack",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Toggles TCP delayed acknowledgements (ACKs).",
                synopsis=("TCP::delayed_ack BOOL_VALUE",),
                snippet=(
                    "Enables or disables TCP delayed acknowledgements.\n"
                    "When enabled, minimizes acknowledgment traffic from BIG-IP by waiting 100ms for additional data to arrive, allowing aggregated ACKs. Can have negative performance implications for some remote hosts depending on their congestion control implementation."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '    log local0. "Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port]."\n'
                    "    # Set client-side delayed ACKs to enabled.\n"
                    "    clientside {\n"
                    "        TCP::delayed_ack enable\n"
                    "    }\n"
                    "    # Set server-side delayed ACKs to disabled.\n"
                    "    serverside {\n"
                    "        TCP::delayed_ack disable\n"
                    "    }\n"
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::delayed_ack BOOL_VALUE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
