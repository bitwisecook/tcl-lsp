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
"""HTTP::redirect -- Redirects an HTTP request or response to the specified URL."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__redirect.html"


@register
class HttpRedirectCommand(CommandDef):
    name = "HTTP::redirect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::redirect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Redirects an HTTP request or response to the specified URL.",
                synopsis=("HTTP::redirect REDIRECT_URL",),
                snippet=(
                    "Redirects an HTTP request or response to the specified URL. Note that\n"
                    "this command sends the response to the client immediately. Therefore,\n"
                    "you cannot specify this command multiple times in an iRule, nor can you\n"
                    "specify any other commands that modify header or content after you\n"
                    "specify this command.\n"
                    "This command will always use a 302 response code. If you wish to use a\n"
                    "different one (e.g. 301), you will need to craft a response using\n"
                    "[HTTP::respond].\n"
                    "If the client is a typical web browser, it will reflect the new URL\n"
                    "that you specify."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    "  if { [HTTP::status] == 404} {\n"
                    '    HTTP::redirect "http://www.example.com/newlocation.html"\n'
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::redirect REDIRECT_URL",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            taint_output_sink="IRULE3004",
            event_requires=EventRequires(
                transport="tcp",
                profiles=frozenset({"HTTP", "FASTHTTP"}),
                also_in=frozenset({"LB_FAILED", "NAME_RESOLVED"}),
            ),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.RESPONSE_COMMIT,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
