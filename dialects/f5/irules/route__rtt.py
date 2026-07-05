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
"""ROUTE::rtt -- Returns the cached round-trip-time estimate."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ROUTE__rtt.html"


@register
class RouteRttCommand(CommandDef):
    name = "ROUTE::rtt"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ROUTE::rtt",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the cached round-trip-time estimate.",
                synopsis=("ROUTE::rtt DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",),
                snippet=(
                    "Returns the cached round-trip-time for the destination and/or\n"
                    "gateway if the relevant TCP profile enables cmetrics-cache.\n"
                    "\n"
                    "The return value only applies to the TMM executing the command. It\n"
                    "does not consider cache entries on other TMMs.\n"
                    "\n"
                    "ROUTE::rtt returns a value of 0 when there are no statistics available.\n"
                    "\n"
                    "NOTE: The returned value is scaled to units of 100ns; to express it\n"
                    "in the same units as TCP::rtt multiply it by 32/10000.\n"
                    "\n"
                    "NOTE: When used with the fastL4 profile, RTT from client/server\n"
                    "needs to be enabled and the client and server need to be using TCP\n"
                    "timestamps."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '    log local0. "Cached rtt is: [ROUTE::rtt [IP::remote_addr]]"\n'
                    "}"
                ),
                return_value="RTT in units of 100ns.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ROUTE::rtt DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",
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
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
