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
"""HTTP::disable -- Changes the HTTP filter from full parsing to passthrough mode."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__disable.html"


_av = make_av(_SOURCE)


@register
class HttpDisableCommand(CommandDef):
    name = "HTTP::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Changes the HTTP filter from full parsing to passthrough mode.",
                synopsis=("HTTP::disable (discard)?",),
                snippet=(
                    "Changes the HTTP filter from full parsing to passthrough mode. This\n"
                    "command is useful when using an HTTP profile with an application that\n"
                    "proxies data over HTTP. One use of this command is when you need to\n"
                    "tunnel PPP over HTTP and disable HTTP processing once the connection\n"
                    "has been established."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "persist hash $key\n"
                    'if { [string toupper [HTTP::method]] eq "CONNECT" } {\n'
                    "\n"
                    "      # Proxy connect method should continue as a passthrough\n"
                    "      HTTP::disable\n"
                    "\n"
                    "      # Ask the server to close the connection after this request\n"
                    "      HTTP::header replace Connection close\n"
                    "   }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::disable (discard)?",
                    arg_values={
                        0: (_av("discard", "HTTP::disable discard", "HTTP::disable (discard)?"),)
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_HEADER,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
