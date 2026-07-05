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
"""UDP::server_port -- Returns the UDP port/service number of a server system."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/UDP__server_port.html"


@register
class UdpServerPortCommand(CommandDef):
    name = "UDP::server_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="UDP::server_port",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the UDP port/service number of a server system.",
                synopsis=("UDP::server_port",),
                snippet=(
                    "Returns the UDP port/service number of the server. This command is\n"
                    "equivalent to the command serverside { UDP::remote_port }."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "    set client [IP::client_addr]:[UDP::client_port]\n"
                    "    set node [IP::server_addr]:[UDP::server_port]\n"
                    '    log local0. "client: $client, server: $server"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="UDP::server_port",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                transport="udp",
                also_in=frozenset(
                    {"SIP_REQUEST", "SIP_REQUEST_SEND", "SIP_RESPONSE", "STREAM_MATCHED"}
                ),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UDP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
