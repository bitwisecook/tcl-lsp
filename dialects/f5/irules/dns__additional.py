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
"""DNS::additional -- Returns, inserts, removes, or clears RRs from the additional section."""

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

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__additional.html"


_av = make_av(_SOURCE)


@register
class DnsAdditionalCommand(CommandDef):
    name = "DNS::additional"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::additional",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns, inserts, removes, or clears RRs from the additional section.",
                synopsis=("DNS::additional ('clear' | (('insert' | 'remove') RR_OBJECT))?",),
                snippet=(
                    "This iRules command returns, inserts, removes, or clears RRs from the\n"
                    "additional section.\n"
                    "\n"
                    "Note: This command functions only in the context of LTM iRules and\n"
                    "requires the DNS Profile, which is only enabled as part of GTM or the\n"
                    "DNS Services add-on."
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
                    synopsis="DNS::additional ?clear | insert <rr> | remove <rr>?",
                    arg_values={
                        0: (
                            _av("clear", "Clear all additional RRs.", "DNS::additional clear"),
                            _av(
                                "insert",
                                "Insert an RR into the additional section.",
                                "DNS::additional insert <rr_object>",
                            ),
                            _av(
                                "remove",
                                "Remove an RR from the additional section.",
                                "DNS::additional remove <rr_object>",
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
                    detail="Clear all additional RRs.",
                    synopsis="DNS::additional clear",
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
                    detail="Insert an RR into the additional section.",
                    synopsis="DNS::additional insert <rr_object>",
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
                    detail="Remove an RR from the additional section.",
                    synopsis="DNS::additional remove <rr_object>",
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
