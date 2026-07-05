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
"""DNS::return -- Skips all further processing after TCL execution and sends the DNS packet in the opposite direction."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__return.html"


@register
class DnsReturnCommand(CommandDef):
    name = "DNS::return"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::return",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Skips all further processing after TCL execution and sends the DNS packet in the opposite direction.",
                synopsis=("DNS::return",),
                snippet=(
                    "This iRules command skips all further processing after TCL execution\n"
                    "and sends the DNS packet in the opposite direction.\n"
                    "In the DNS_REQUEST context, DNS::return signals that no further DNS\n"
                    "resolution should occur for this request upon completion of the event.\n"
                    "To provide a useful response, resource record and header changes must\n"
                    "be made in the iRule. The next event triggered is the DNS_RESPONSE\n"
                    "event.\n"
                    "In the DNS_RESPONSE context, DNS::return sends a request back for\n"
                    "additional processing.\n"
                    "\n"
                    "**Important**: `DNS::return` does NOT stop iRule execution. You must\n"
                    "follow it with `return` to prevent the rest of the event handler\n"
                    "from running."
                ),
                source=_SOURCE,
                examples=(
                    "# Send one or more IP addresses for a response to an A query\n"
                    "# Use on an LTM virtual server with a DNS profile enabled\n"
                    "when DNS_REQUEST {\n"
                    "        # Log query details\n"
                    '        log local0. "\\[DNS::question name\\]: [DNS::question name],\\\n'
                    "                \\[DNS::question class\\]: [DNS::question class],\n"
                    '                \\[DNS::question type\\]: [DNS::question type]"\n'
                    "\n"
                    "        # Generate an answer with two A records"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::return",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DNS"})),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DNS_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
