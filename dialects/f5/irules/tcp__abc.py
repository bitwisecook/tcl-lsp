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
"""TCP::abc -- Toggles Appropriate Byte Counting."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__abc.html"


@register
class TcpAbcCommand(CommandDef):
    name = "TCP::abc"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::abc",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Toggles Appropriate Byte Counting.",
                synopsis=("TCP::abc BOOL_VALUE",),
                snippet="This command will enable or disable TCP Appropriate Byte Counting. Increases congestion window in accordance with bytes actually acknowledged, rather than allowing small acknowledgements to increase the window by an entire segment.",
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '    log local0. "Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port]."\n'
                    "    # If an HTTP connection, enable ABC on the client side and\n"
                    "    # disable ABC on the server side.\n"
                    "    if { [server_port] == 80 } {\n"
                    "        clientside {\n"
                    "            TCP::abc enable\n"
                    '            log local0. "Client MSS: [TCP::mss]"\n'
                    "        }\n"
                    "        serverside {\n"
                    "            TCP::abc disable\n"
                    '            log local0. "Server MSS: [TCP::mss]"\n'
                    "        }\n"
                    "    }\n"
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::abc BOOL_VALUE",
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
