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
"""URI::compare -- Compares two URI's for equality."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/URI__compare.html"


@register
class UriCompareCommand(CommandDef):
    name = "URI::compare"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="URI::compare",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Compares two URI's for equality.",
                synopsis=("URI::compare URI_STRING URI_STRING",),
                snippet=(
                    "Compares two URI's as recommended by RFC2616 section 3.2.3.\n"
                    "\n"
                    "3.2.3 URI Comparison\n"
                    "\n"
                    "   When comparing two URIs to decide if they match or not, a client\n"
                    "   SHOULD use a case-sensitive octet-by-octet comparison of the entire\n"
                    "   URIs, with these exceptions:\n"
                    "\n"
                    "      - A port that is empty or not given is equivalent to the default\n"
                    "        port for that URI-reference;\n"
                    "\n"
                    "        - Comparisons of host names MUST be case-insensitive;\n"
                    "\n"
                    "        - Comparisons of scheme names MUST be case-insensitive;\n"
                    "\n"
                    '        - An empty abs_path is equivalent to an abs_path of "/".'
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '  set uri_to_check "/dir1/somepath"\n'
                    "  if { [URI::compare [HTTP::uri] $uri_to_check] } {\n"
                    '    log local0. "URI\'s are equal!"\n'
                    "  }\n"
                    "}"
                ),
                return_value="Returns 1 if URIs match; 0 otherwise.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="URI::compare URI_STRING URI_STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_URI,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
