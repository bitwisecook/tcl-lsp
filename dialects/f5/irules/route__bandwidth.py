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
"""ROUTE::bandwidth -- Returns a bandwidth estimate for a destination derived from entries in the congestion metrics cache."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ROUTE__bandwidth.html"


@register
class RouteBandwidthCommand(CommandDef):
    name = "ROUTE::bandwidth"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ROUTE::bandwidth",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a bandwidth estimate for a destination derived from entries in the congestion metrics cache.",
                synopsis=("ROUTE::bandwidth DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",),
                snippet=(
                    "Returns a bandwidth estimate for a destination derived from\n"
                    "entries in the congestion metrics cache.\n"
                    "\n"
                    "As of v12.0, divides the cached congestion window (cwnd) value\n"
                    "by the cached round-trip-time (RTT ) to obtain a bandwidth\n"
                    "estimate in kbps. If there is no entry, it returns 0.\n"
                    "\n"
                    "Note: The return value only applies to the TMM executing the command.\n"
                    "It does not consider cache entries on other TMMs."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    if { [ROUTE::bandwidth [IP::remote_addr]] > 0 } {\n"
                    '        log local0. "cached bandwidth is: [ROUTE::bandwidth [IP::remote_addr]]"\n'
                    "    }\n"
                    "}"
                ),
                return_value="The bandwidth estimate to the destination and/or gateway in kbps.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ROUTE::bandwidth DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",
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
