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

# Curated community template proc.
"""xff_list -- Return sorted unique valid IPs from an X-Forwarded-For header."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/xff_list.html"


@register
class XffListProc(CommandDef):
    name = "xff_list"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="xff_list",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary=(
                    "Return a sorted, deduplicated list of valid non-loopback "
                    "IP addresses from the X-Forwarded-For header."
                ),
                synopsis=(
                    "call xff_list",
                    'call xff_list "X-Real-IP"',
                ),
                snippet=(
                    "Also known as `xff_uniq_sorted_ip_list`.  Collects all "
                    "addresses from the named header (default `X-Forwarded-For`), "
                    "even when the header appears multiple times.\n"
                    "\n"
                    "  - Entries that are not IPv4 or IPv6 are removed\n"
                    "  - Both IPv4 and IPv6 addresses are collected and returned\n"
                    "  - The result is sorted; duplicate IPs are collapsed\n"
                    "  - FQDNs are not valid IPs and are therefore removed\n"
                    "  - Loopback / zero addresses (`127.0.0.0/8`, "
                    "`0.0.0.0/32`, `::/127`) are filtered out\n"
                    "\n"
                    "Useful for geolocation or blacklist lookups against "
                    "forwarded addresses."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST priority 350 {\n"
                    "    foreach ip [call xff_list] {\n"
                    '        if {[class match -- $ip eq "blacklist-ips"]} {\n'
                    '            log local0. "Blocking bad XFF IP: $ip"\n'
                    "            reject\n"
                    "            return\n"
                    "        }\n"
                    "    }\n"
                    "}"
                ),
                return_value="A Tcl list of unique, sorted IP address strings.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="xff_list ?xff_header_name?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0, 1),
            ),
            event_requires=EventRequires(
                transport="tcp",
                profiles=frozenset({"HTTP"}),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_HEADER,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
