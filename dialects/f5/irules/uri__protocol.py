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
"""URI::protocol -- Returns the protocol of the given URI."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/URI__protocol.html"


@register
class UriProtocolCommand(CommandDef):
    name = "URI::protocol"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="URI::protocol",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the protocol of the given URI.",
                synopsis=("URI::protocol URI_STRING",),
                snippet="Returns the protocol of the given URI.",
                source=_SOURCE,
                examples=(
                    "when RULE_INIT {\n"
                    "        # Loop through some test URLs and URIs and log the URI::protocol value\n"
                    "        foreach uri [list \\\n"
                    "                http://test.com \\\n"
                    "                https://test.com \\\n"
                    "                ftp://test.com \\\n"
                    "                sip://test.com \\\n"
                    "                myproto://test.com \\\n"
                    "                /test.com \\\n"
                    "                /uri?url=http://test.example.com/uri \\\n"
                    "        ] {\n"
                    '                log local0. "\\[URI::protocol $uri\\]: [URI::protocol $uri]"\n'
                    "        }\n"
                    "}"
                ),
                return_value="Returns the protocol of the given URI.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="URI::protocol URI_STRING",
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
