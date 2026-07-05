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
"""MR::peer -- Defines a peer to use for routing a message to."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__peer.html"


_av = make_av(_SOURCE)


@register
class MrPeerCommand(CommandDef):
    name = "MR::peer"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::peer",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Defines a peer to use for routing a message to.",
                synopsis=(
                    "MR::peer PEER (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG))",
                ),
                snippet="The MR::peer command defines a peer to use for routing a message to. The peer may either refer to a named pool or a tuple (IP address, port and route domain iD). When creating a connection to a peer, the parameters of either a virtual server or a transport config object will be used. The peer object will only exist in the current connections connflow. When adding a route (via MR::route add), it will first look for a locally created peer object then for a peer object from the configuration. Once the current connection closes, the local peer object will go away.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '    MR::peer self_peer config tc1 host "[IP::remote_addr]:[TCP::remote_port]"\n'
                    '    GENERICMESSAGE::route add dest "[IP::remote_addr]" peer self_peer\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::peer PEER (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG))",
                    arg_values={
                        0: (
                            _av(
                                "virtual",
                                "MR::peer virtual",
                                "MR::peer PEER (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG))",
                            ),
                            _av(
                                "config",
                                "MR::peer config",
                                "MR::peer PEER (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG))",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
