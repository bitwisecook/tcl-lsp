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
"""DNS::class -- Gets or sets the resource record class field."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__class.html"


@register
class DnsClassCommand(CommandDef):
    name = "DNS::class"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::class",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the resource record class field.",
                synopsis=("DNS::class RR_OBJECT (DNS_CLASS)?",),
                snippet=(
                    "This iRules command gets or sets the resource record class field\n"
                    "\n"
                    "Note: This command requires the DNS Profile, which is only enabled as\n"
                    "part of GTM or the DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "ass of all response records to IN (internet)\n"
                    "            when DNS_RESPONSE {\n"
                    "                set rrs [DNS::answer]\n"
                    "                foreach rr $rrs {\n"
                    "                    DNS::class $rr IN\n"
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::class RR_OBJECT (DNS_CLASS)?",
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
