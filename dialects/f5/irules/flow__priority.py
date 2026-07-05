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
"""FLOW::priority -- Set/Get flow's internal packet priority."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/FLOW__priority.html"


_av = make_av(_SOURCE)


@register
class FlowPriorityCommand(CommandDef):
    name = "FLOW::priority"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="FLOW::priority",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set/Get flow's internal packet priority.",
                synopsis=(
                    "FLOW::priority FLOW_PRIORITY",
                    "FLOW::priority (clientside | serverside) (FLOW_PRIORITY)?",
                    "FLOW::priority (ANY_CHARS) (FLOW_PRIORITY)?",
                ),
                snippet=(
                    "This command is used to get/set the flow's internal packet priority.\n"
                    "Valid priority is any integer value from 0 to 7.\n"
                    "Syntax:\n"
                    "FLOW::priority [TCL handle|clientside|serverside] [priority]\n"
                    "\n"
                    "Following are the variations of this command:\n"
                    "\n"
                    "FLOW::priority\n"
                    "\n"
                    " Returns the internal packet priority of current flow.\n"
                    "\n"
                    "Flow::priority <priority>\n"
                    "\n"
                    " Sets the priority of the current flow's internal packet priority.\n"
                    " Exception is thrown if priority is outside the allowed range [0-7].\n"
                    "\n"
                    "FLOW::priority clientside\n"
                    "\n"
                    " Returns the priority of the clientside flow's internal packet priority."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "  FLOW::priority serverside 4\n"
                    "\n"
                    "  # Alternate way to use the command using the TCL flow handle.\n"
                    "  # Set priority on both client side and server side flow.\n"
                    "  FLOW::priority $clientflow 2\n"
                    "  FLOW::priority [FLOW::this] 2\n"
                    "}"
                ),
                return_value="Get operation returns an integer between 0-7. Set operation returns nothing. Exception is thrown if the operation cannot be completed.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="FLOW::priority FLOW_PRIORITY",
                    arg_values={
                        0: (
                            _av(
                                "clientside",
                                "FLOW::priority clientside",
                                "FLOW::priority (clientside | serverside) (FLOW_PRIORITY)?",
                            ),
                            _av(
                                "serverside",
                                "FLOW::priority serverside",
                                "FLOW::priority (clientside | serverside) (FLOW_PRIORITY)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"FLOW"}),
                also_in=frozenset({"CLIENT_ACCEPTED", "SERVER_CONNECTED"}),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FLOW_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
