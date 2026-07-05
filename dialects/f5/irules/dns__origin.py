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
"""DNS::origin -- Returns the originator of the DNS message."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__origin.html"


@register
class DnsOriginCommand(CommandDef):
    name = "DNS::origin"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::origin",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the originator of the DNS message.",
                synopsis=("DNS::origin",),
                snippet=(
                    "Returns the last module to modify the DNS message. Return values:\n"
                    "\n"
                    "CLIENT\n"
                    "\n"
                    "   This message has just been received by the BigIP from a client's query,\n"
                    "   and nothing has been processed.\n"
                    "\n"
                    "SERVER\n"
                    "\n"
                    "   This message has just been received by the BigIP from a server's\n"
                    "   response to a DNS query, such as On-Box or Off-Box BIND, or another\n"
                    "   BigIP entirely.\n"
                    "\n"
                    "CACHE\n"
                    "\n"
                    "   This message is a response from the DNS Cache.\n"
                    "\n"
                    "RPZ\n"
                    "\n"
                    "   This message is a response from the Response Policy Zone in your BigIP.\n"
                    "   It was blocked and either NXDOMAIN or a Walled Garden was returned as a\n"
                    "   response."
                ),
                source=_SOURCE,
                examples=(
                    "equests that were not resolved by DNS Express\n"
                    "            when DNS_RESPONSE {\n"
                    '                if { [DNS::origin] ne "DNSX" } {\n'
                    "                    DNS::drop\n"
                    "                }\n"
                    "            }"
                ),
                return_value="CLIENT SERVER CACHE GTM_BUILD GTM_REWRITE DNSX DNSSEC LAST_ACTION TCL RPZ RATE_LIMITER",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::origin",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DNS"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DNS_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
