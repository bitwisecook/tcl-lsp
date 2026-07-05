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
"""UDP::client_port -- Returns the UDP port/service number of a client system."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/UDP__client_port.html"


@register
class UdpClientPortCommand(CommandDef):
    name = "UDP::client_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="UDP::client_port",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the UDP port/service number of a client system.",
                synopsis=("UDP::client_port",),
                snippet=(
                    "Returns the UDP port/service number of the client system. This command\n"
                    "is equivalent to the command clientside { UDP::remote_port }."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_DATA {\n"
                    '  if { [UDP::payload 50] contains "XYZ" } {\n'
                    "    pool xyz_servers\n"
                    '    persist uie "[IP::client_addr]:[UDP::client_port]" 300\n'
                    "  } else {\n"
                    "    pool web_servers\n"
                    "  }\n"
                    "}"
                ),
                return_value="Returns the UDP port/service number of the client system",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="UDP::client_port",
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
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
