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
"""LB::status -- Returns the status of a node address or pool member."""

# Introduced: BIG-IP v9+ (core load-balancing iRules command) (approximate, from F5 documentation)

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__status.html"


_av = make_av(_SOURCE)

_READ_EFFECT = (
    SideEffect(
        target=SideEffectTarget.POOL_SELECTION, reads=True, connection_side=ConnectionSide.SERVER
    ),
)


@register
class LbStatusCommand(CommandDef):
    name = "LB::status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::status",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the status of a node address or pool member.",
                synopsis=(
                    "LB::status (LB_STATUS)?",
                    "LB::status node IP_ADDR (LB_STATUS)?",
                    "LB::status pool POOL_OBJ member IP_ADDR PORT (LB_STATUS)?",
                ),
                snippet=(
                    "Returns the status of a node address or pool member. Possible status values are up, down, session_enabled, and session_disabled. If you supply no arguments, returns the status of the currently-selected pool member.\n"
                    "Syntax:\n"
                    "    LB::status\n"
                    "    LB::status node <address>\n"
                    "    LB::status pool <pool name> member <IP address> <port>\n"
                    "    LB::status <up | down | session_enabled | session_disabled>\n"
                    "    LB::status node <address> <up | down | session_enabled | session_disabled>\n"
                    "    LB::status pool <pool name> member <address> <port> <up | down | session_enabled | session_disabled>"
                ),
                source=_SOURCE,
                examples=(
                    "when LB_FAILED {\n"
                    '    if { [LB::status pool $poolname member $ip $port] eq "down" } {\n'
                    '        log "Server $ip $port down!"\n'
                    "    }\n"
                    "}"
                ),
                return_value="LB::status Returns the status of the currently-selected node (after LB_SELECTED event only). Possible values are: up | down | session_enabled | session_disabled",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::status ?node <addr> | pool <pool> member <addr> <port>? ?status?",
                    arg_values={
                        0: (
                            _av(
                                "node", "Query/set node status.", "LB::status node <addr> ?status?"
                            ),
                            _av(
                                "pool",
                                "Query/set pool member status.",
                                "LB::status pool <pool> member <addr> <port> ?status?",
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
                    target=SideEffectTarget.POOL_SELECTION,
                    reads=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
            subcommands={
                "node": SubCommand(
                    name="node",
                    arity=Arity(1),
                    detail="Query/set node status.",
                    synopsis="LB::status node <addr> ?status?",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "pool": SubCommand(
                    name="pool",
                    arity=Arity(3),
                    detail="Query/set pool member status.",
                    synopsis="LB::status pool <pool> member <addr> <port> ?status?",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
            },
        )
