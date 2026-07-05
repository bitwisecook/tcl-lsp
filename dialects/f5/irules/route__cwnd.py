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
"""ROUTE::cwnd -- Returns the cached congestion window (cwnd) value."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ROUTE__cwnd.html"


@register
class RouteCwndCommand(CommandDef):
    name = "ROUTE::cwnd"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ROUTE::cwnd",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the cached congestion window (cwnd) value.",
                synopsis=("ROUTE::cwnd DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",),
                snippet=(
                    "Returns the cached congestion window (cwnd) value for a given\n"
                    "destination IP and/or gateway.\n"
                    "\n"
                    "The return value only applies to the TMM executing the command. It\n"
                    "does not consider cache entries on other TMMs."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set cwnd [ROUTE::cwnd [IP::remote_addr]]\n"
                    "    if { $cwnd > 0 } {\n"
                    '        log local0. "Destination found in cache. Initializing cwnd to $cwnd"\n'
                    "    } else {\n"
                    '        log local0. "Destination not found in cache."\n'
                    "    }\n"
                    "}"
                ),
                return_value="The cached congestion window in bytes.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ROUTE::cwnd DESTINATION_IP_ADDRESS (GATEWAY_IP_ADDRESS)?",
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
