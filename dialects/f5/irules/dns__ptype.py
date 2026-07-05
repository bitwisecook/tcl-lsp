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
"""DNS::ptype -- Returns the type of the DNS packet."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__ptype.html"


@register
class DnsPtypeCommand(CommandDef):
    name = "DNS::ptype"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::ptype",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the type of the DNS packet.",
                synopsis=("DNS::ptype",),
                snippet=(
                    "This iRules command returns the type of the DNS packet.\n"
                    "\n"
                    "Note: This command requires the DNS Profile, which is only enabled as\n"
                    "part of GTM or the DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "OMAIN response is going to be sent,\n"
                    "            # instead attach a record to resolve to.\n"
                    "            when DNS_RESPONSE {\n"
                    '                if { [DNS::ptype] == "NXDOMAIN" } {\n'
                    "                    DNS::header rcode NOERROR\n"
                    '                    DNS::answer insert "[DNS::question name]. 60 [DNS::question class] [DNS::question type] 192.168.1.245"\n'
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::ptype",
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
