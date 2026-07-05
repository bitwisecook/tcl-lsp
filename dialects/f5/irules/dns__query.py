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
"""DNS::query -- Returns or constructs and sends a query to the DNS-Express database for a name and type."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__query.html"


_av = make_av(_SOURCE)


@register
class DnsQueryCommand(CommandDef):
    name = "DNS::query"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::query",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or constructs and sends a query to the DNS-Express database for a name and type.",
                synopsis=("DNS::query ('dnsx' | 'dns-express') NAME DNS_TYPE (DNSSEC)?",),
                snippet=(
                    "This iRules command returns or constructs and sends a query to the\n"
                    "DNS-Express database for a name and type (IN class only).\n"
                    "\n"
                    "Note: This command requires the DNS Profile, which is only enabled as\n"
                    "part of GTM or the DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "when DNS_RESPONSE {\n"
                    "        set rrsl [DNS::query dnsx nameserver.org SOA]\n"
                    "        foreach rrs $rrsl {\n"
                    "            foreach rr $rrs {\n"
                    '                if { [DNS::type $rr] == "SOA" } {\n'
                    "                    DNS::additional insert $rr\n"
                    "                }\n"
                    "            }\n"
                    "        }\n"
                    "    }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::query ('dnsx' | 'dns-express') NAME DNS_TYPE (DNSSEC)?",
                    arg_values={
                        0: (
                            _av(
                                "dnsx",
                                "DNS::query dnsx",
                                "DNS::query ('dnsx' | 'dns-express') NAME DNS_TYPE (DNSSEC)?",
                            ),
                            _av(
                                "dns-express",
                                "DNS::query dns-express",
                                "DNS::query ('dnsx' | 'dns-express') NAME DNS_TYPE (DNSSEC)?",
                            ),
                        )
                    },
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

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
