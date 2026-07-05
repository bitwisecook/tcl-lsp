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
"""DNS::authority -- Returns, inserts, removes, or clears RRs from the authority section."""

# Introduced: BIG-IP v10+ (core DNS iRules command) (approximate, from F5 documentation)

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

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__authority.html"


_av = make_av(_SOURCE)


@register
class DnsAuthorityCommand(CommandDef):
    name = "DNS::authority"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::authority",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns, inserts, removes, or clears RRs from the authority section.",
                synopsis=("DNS::authority ('clear' | (('insert' | 'remove') RR_OBJECT))?",),
                snippet=(
                    "This iRules command returns, inserts, removes, or clears RRs from the\n"
                    "authority section.\n"
                    "\n"
                    "Note: This command functions only in the context of LTM iRules and\n"
                    "requires the DNS Profile, which is only enabled as part of GTM or the\n"
                    "DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "authority record in all responses\n"
                    "            when DNS_RESPONSE {\n"
                    '                DNS::authority insert [DNS::rr "devcentral.f5.com. 88 IN SOA 1.2.3.4"]\n'
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::authority ?clear | insert <rr> | remove <rr>?",
                    arg_values={
                        0: (
                            _av("clear", "Clear all authority RRs.", "DNS::authority clear"),
                            _av(
                                "insert",
                                "Insert an RR into the authority section.",
                                "DNS::authority insert <rr_object>",
                            ),
                            _av(
                                "remove",
                                "Remove an RR from the authority section.",
                                "DNS::authority remove <rr_object>",
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
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
            subcommands={
                "clear": SubCommand(
                    name="clear",
                    arity=Arity(0, 0),
                    detail="Clear all authority RRs.",
                    synopsis="DNS::authority clear",
                    mutator=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.DNS_STATE,
                            writes=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
                "insert": SubCommand(
                    name="insert",
                    arity=Arity(1, 1),
                    detail="Insert an RR into the authority section.",
                    synopsis="DNS::authority insert <rr_object>",
                    mutator=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.DNS_STATE,
                            writes=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
                "remove": SubCommand(
                    name="remove",
                    arity=Arity(1, 1),
                    detail="Remove an RR from the authority section.",
                    synopsis="DNS::authority remove <rr_object>",
                    mutator=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.DNS_STATE,
                            writes=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
            },
        )
