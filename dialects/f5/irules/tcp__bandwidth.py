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
"""TCP::bandwidth -- Returns the estimated bandwidth of the connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__bandwidth.html"


@register
class TcpBandwidthCommand(CommandDef):
    name = "TCP::bandwidth"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::bandwidth",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns the estimated bandwidth of the connection.",
                synopsis=("TCP::bandwidth",),
                snippet=(
                    'Returns the estimated bandwidth measured as "congestion window" / "the measured round trip time".\n'
                    "The values returned are only estimates, and can vary even during the connection.\n"
                    "\n"
                    "Note: Starting with BIG-IP v9.4.2, client bandwidth calculations are unavailable, always returning 0. Starting with BIG-IP v12.0 nonzero values are returned."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_DATA {\n"
                    'if {[HTTP::uri] starts_with "/xxx.css"}{\n'
                    "      set bandwidth [TCP::bandwidth]\n"
                    "      if {$bandwidth < XXX} {\n"
                    '         HTTP::uri "/boring-xxx.css"\n'
                    "      }\n"
                    "   }\n"
                    "}"
                ),
                return_value="The estimated bandwidth in kilobits per second.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::bandwidth",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                transport="tcp",
                also_in=frozenset({"SIP_REQUEST", "SIP_REQUEST_SEND", "SIP_RESPONSE"}),
            ),
            cse_candidate=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                    scope=StorageScope.EVENT,
                ),
            ),
        )
