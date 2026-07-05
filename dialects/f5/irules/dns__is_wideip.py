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
"""DNS::is_wideip -- Returns status (true/false) if a string is a configured wide IP."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNS__is_wideip.html"


@register
class DnsIsWideipCommand(CommandDef):
    name = "DNS::is_wideip"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNS::is_wideip",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns status (true/false) if a string is a configured wide IP.",
                synopsis=("DNS::is_wideip DNS_STRING",),
                snippet=(
                    "This iRules command returns status (true/false) if a string is a\n"
                    "configured wide IP.\n"
                    "\n"
                    "Note: This command functions only in the context of LTM iRules and\n"
                    "requires the DNS Profile, which is only enabled as part of GTM or the\n"
                    "DNS Services add-on."
                ),
                source=_SOURCE,
                examples=(
                    "when DNS_REQUEST {\n"
                    "  if { [DNS::is_wideip [DNS::question name]] } {\n"
                    '    log local0. "[DNS::question name] is a wideIP"\n'
                    "  } else {\n"
                    '      log local0. "[DNS::question name] is not a wideIP"\n'
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNS::is_wideip DNS_STRING",
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
