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
"""DNS::scrape -- Allows users to walk over a DNS message and parse out information from the packet based on user supplied arguments."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__scrape.html"


_av = make_av(_SOURCE)


@register
class DnsScrapeCommand(CommandDef):
    name = "DNS::scrape"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::scrape",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Allows users to walk over a DNS message and parse out information from the packet based on user supplied arguments.",
                synopsis=(
                    "DNS::scrape ('AUTHORITY' | 'ADDITIONAL' | 'ANSWER' | 'ALL') (DNS_SCRAPE_VAL)+",
                ),
                snippet=(
                    "This iRules command allows users to walk over a DNS message and parse\n"
                    "out information from the packet based on user supplied arguments.\n"
                    "\n"
                    "Note: This command functions only in the context of LTM iRules and\n"
                    "requires the DNS Profile, which is only enabled as part of GTM or the\n"
                    "DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "when DNS_RESPONSE {\n"
                    "   foreach rr [DNS::scrape ANSWER type ttl qnamelen rdatalen] {\n"
                    '     log local2. "ANSWER: $rr"\n'
                    "   }\n"
                    "   foreach rr [DNS::scrape AUTHORITY type ttl class qnamelen rdatalen] {\n"
                    '     log local2. "AUTHORITY: $rr"\n'
                    "   }\n"
                    "   foreach rr [DNS::scrape ADDITIONAL type ttl class qnamelen rdatalen] {\n"
                    '     log local2. "ADDITIONAL: $rr"\n'
                    "   }\n"
                    " }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::scrape ('AUTHORITY' | 'ADDITIONAL' | 'ANSWER' | 'ALL') (DNS_SCRAPE_VAL)+",
                    arg_values={
                        0: (
                            _av(
                                "AUTHORITY",
                                "DNS::scrape AUTHORITY",
                                "DNS::scrape ('AUTHORITY' | 'ADDITIONAL' | 'ANSWER' | 'ALL') (DNS_SCRAPE_VAL)+",
                            ),
                            _av(
                                "ADDITIONAL",
                                "DNS::scrape ADDITIONAL",
                                "DNS::scrape ('AUTHORITY' | 'ADDITIONAL' | 'ANSWER' | 'ALL') (DNS_SCRAPE_VAL)+",
                            ),
                            _av(
                                "ANSWER",
                                "DNS::scrape ANSWER",
                                "DNS::scrape ('AUTHORITY' | 'ADDITIONAL' | 'ANSWER' | 'ALL') (DNS_SCRAPE_VAL)+",
                            ),
                            _av(
                                "ALL",
                                "DNS::scrape ALL",
                                "DNS::scrape ('AUTHORITY' | 'ADDITIONAL' | 'ANSWER' | 'ALL') (DNS_SCRAPE_VAL)+",
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
        )
