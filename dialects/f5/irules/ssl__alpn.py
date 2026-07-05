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
"""SSL::alpn -- Handle the ALPN TLS extension."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__alpn.html"


@register
class SslAlpnCommand(CommandDef):
    name = "SSL::alpn"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::alpn",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Handle the ALPN TLS extension.",
                synopsis=(
                    "SSL::alpn set (ARG)+",
                    "SSL::alpn",
                ),
                snippet=(
                    "Sets or retrieves the Application Layer Protocol Negotiation (ALPN) string.\n"
                    "\n"
                    "SSL::alpn\n"
                    "  Retrieve the selected ALPN string\n"
                    "\n"
                    "SSL::alpn set str1[ str2...]\n"
                    "  Set the advertised ALPN string"
                ),
                source=_SOURCE,
                examples=(
                    'when CLIENTSSL_CLIENTHELLO {\n    SSL::alpn set "spdy/1" "spdy/2" "http/2"\n}'
                ),
                return_value="SSL::alpn Returns the negotiated ALPN string SSL::alpn set ... There is no return value.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::alpn set (ARG)+",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                client_side=True, transport="tcp", profiles=frozenset({"CLIENTSSL"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
