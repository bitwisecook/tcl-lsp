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
"""uniq_sorted_ip_list -- Sorted unique valid IPs from arbitrary input."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/uniq_sorted_ip_list.html"


@register
class UniqSortedIpListProc(CommandDef):
    name = "uniq_sorted_ip_list"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="uniq_sorted_ip_list",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary=(
                    "Return a sorted, deduplicated list of valid IP addresses "
                    "extracted from the given arguments."
                ),
                synopsis=(
                    "call uniq_sorted_ip_list $ip_string",
                    "call uniq_sorted_ip_list 1.1.1.1 {2.2.2.2, 3.3.3.3} 5.5.5.5",
                ),
                snippet=(
                    "Like `xff_list` but takes a list of potential IPs as an "
                    "argument rather than reading from an HTTP header.\n"
                    "\n"
                    "The list may be nested and may contain commas or spaces "
                    "as delimiters.\n"
                    "\n"
                    "  - Entries that are not IPv4 or IPv6 are removed\n"
                    "  - The result is sorted; duplicate IPs are collapsed\n"
                    "  - Both IPv4 and IPv6 addresses are collected and returned\n"
                    "  - FQDNs are not valid IPs and are therefore removed\n"
                    "\n"
                    "Unlike the `xff_*` variants, this proc does **not** filter "
                    "out loopback or zero addresses."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST priority 350 {\n"
                    "    foreach ip [call uniq_sorted_ip_list"
                    " 1.1.1.1 {2.2.2.2, 3.3.3.3}"
                    " 2a01:4b00:8480:ae00:acf0:fe84:3bf2:eeee"
                    " badentry 5.5.5.5] {\n"
                    '        if {[class match -- $ip eq "blacklist-ips"]} {\n'
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
                    synopsis="uniq_sorted_ip_list ?arg ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
