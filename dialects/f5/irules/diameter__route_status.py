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
"""DIAMETER::route_status -- Returns the routing status of the current message."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__route_status.html"


@register
class DiameterRouteStatusCommand(CommandDef):
    name = "DIAMETER::route_status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::route_status",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the routing status of the current message.",
                synopsis=("DIAMETER::route_status",),
                snippet=(
                    "The DIAMETER::route_status command returns the routing status of the current\n"
                    "message. Valid status are:\n"
                    '  * "unprocessed"\n'
                    '  * "route found"\n'
                    '  * "no route found"\n'
                    '  * "dropped"\n'
                    '  * "queue full"\n'
                    '  * "no connection"\n'
                    '  * "connection closing"\n'
                    '  * "internal error"\n'
                    "\n"
                    '"route found" is based on the DIAMETER RouteTable finding a route. It\n'
                    "is not affected by the proxy’s ability to create a connection, so even\n"
                    "if the server is not listening on the specified address or marked\n"
                    'down, it still returns status as "route found" if the RouteTable is\n'
                    "able to find the route."
                ),
                source=_SOURCE,
                return_value="Returns routing status of the current message",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::route_status",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DIAMETER"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
