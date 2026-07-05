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
"""URI::host -- Returns the host portion of a given URI."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/URI__host.html"


@register
class UriHostCommand(CommandDef):
    name = "URI::host"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="URI::host",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the host portion of a given URI.",
                synopsis=("URI::host URI_STRING",),
                snippet="Returns the host portion of a given URI.",
                source=_SOURCE,
                examples=(
                    "when RULE_INIT {\n"
                    "        # Loop through some test URLs and URIs and log the URI::host value\n"
                    "        foreach uri [list \\\n"
                    "                http://example.com/file.ext \\\n"
                    "                http://example.com:80/file.ext \\\n"
                    "                https://example.com:443/file.ext \\\n"
                    "                ftp://example.com/file.ext \\\n"
                    "                sip://example.com/file.ext \\\n"
                    "                myproto://example.com/file.ext \\\n"
                    "                /example.com \\\n"
                    "                /uri?url=http://example.com/uri \\\n"
                    "        ] {"
                ),
                return_value="Returns the host portion of a given URI.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="URI::host URI_STRING",
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

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
