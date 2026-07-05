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
"""DNS::rr -- Creates a new resource record object with specified attributes or as a complete string."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__rr.html"


@register
class DnsRrCommand(CommandDef):
    name = "DNS::rr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::rr",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Creates a new resource record object with specified attributes or as a complete string.",
                synopsis=(
                    "DNS::rr ANY_CHARS",
                    "DNS::rr NAME DNS_TYPE DNS_CLASS TTL RDATA",
                ),
                snippet=(
                    "This iRules command creates a new resource record object with specified\n"
                    "attributes or as a complete string.\n"
                    "\n"
                    "Note: This command requires the DNS Profile, which is only enabled as\n"
                    "part of GTM or the DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "when DNS_RESPONSE {\n"
                    "        set rrs [DNS::answer]\n"
                    "        foreach rr $rrs {\n"
                    "            DNS::ttl $rr 1234\n"
                    "        }\n"
                    '        set new_rr [DNS::rr "bigip3900-30.f5net.com. 88 IN A 1.2.3.4"]\n'
                    "        DNS::additional insert $new_rr\n"
                    "    }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::rr ANY_CHARS",
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
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
