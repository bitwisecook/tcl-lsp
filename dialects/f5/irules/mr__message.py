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
"""MR::message -- Returns or sets details in the message routing table."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__message.html"


@register
class MrMessageCommand(CommandDef):
    name = "MR::message"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::message",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets details in the message routing table.",
                synopsis=(
                    "MR::message clone (CLONE_ID)+",
                    "MR::message clone -count CLONE_COUNT",
                ),
                snippet=(
                    "Clones the message a number of times (one for each CLONE_ID) and dispatches each cloned\n"
                    "message as ingress. After the original message has completed the event in which this command\n"
                    "is executed, each cloned message executes the MR_INGRESS iRule event for itself.\n"
                    "(CLONE_ID)+ can be one or more strings separated by space.\n"
                    "Protection against infinite loops should be considered!\n"
                    "Returns the clone_count, see below, (allowed only at MR_INGRESS).\n"
                    "            \n"
                    "Clones the message CLONE_COUNT number of times and dispatches each cloned\n"
                    "message as ingress."
                ),
                source=_SOURCE,
                examples=(
                    "# Example 1\n"
                    "when MR_INGRESS {\n"
                    "    if {[GENERICMESSAGE::message is_request] != 0} {\n"
                    "        set host [MR::message pick_host peer /Common/mypeer]\n"
                    "        MR::message route config tcp_tc host $host\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::message clone (CLONE_ID)+",
                    options=(OptionSpec(name="-count", detail="Option -count.", takes_value=True),),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
