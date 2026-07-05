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
"""DNS::rpz_policy -- Returns the RPZ policy associated with the DNS cache."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__rpz_policy.html"


@register
class DnsRpzPolicyCommand(CommandDef):
    name = "DNS::rpz_policy"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::rpz_policy",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the RPZ policy associated with the DNS cache.",
                synopsis=("DNS::rpz_policy",),
                snippet=(
                    "Returns the RPZ (Response Policy Zones) policy associated with the DNS cache.\n"
                    "\n"
                    "The possible return values are:\n"
                    '    * "" (empty string) if RPZ is not configured.\n'
                    '    * "NXDOMAIN" if RPZ is configured to return an NXDOMAIN response on a match.\n'
                    '    * "WG <walled garden name>" if RPZ is configured to return a Walled Garden redirect on a match.'
                ),
                source=_SOURCE,
                examples=(
                    "when DNS_RESPONSE {\n"
                    '     if { [DNS::origin] eq "RPZ"} {\n'
                    '        log local0. "[DNS::question name] resulted in an RPZ [DNS::rpz_policy]"\n'
                    "     }\n"
                    "}"
                ),
                return_value='* "" (empty string) if RPZ is not configured. * "NXDOMAIN" if RPZ is configured to return an NXDOMAIN response on a match. * "WG <walled garden name>" if RPZ is configured to return a Walled Garden redirect on a match.',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::rpz_policy",
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
