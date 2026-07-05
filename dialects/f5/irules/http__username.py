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
"""HTTP::username -- Returns the username part of HTTP basic authentication."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__username.html"


@register
class HttpUsernameCommand(CommandDef):
    name = "HTTP::username"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::username",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the username part of HTTP basic authentication.",
                synopsis=("HTTP::username",),
                snippet=(
                    "Returns the username part of HTTP basic authentication.\n"
                    "As described in RFC2617 the username and password in basic\n"
                    "authentication is sent by the client in the Authorization header. The\n"
                    "client base64 encodes the username and password in the format of:\n"
                    "Authorization: Basic base64encoding(username:password)\n"
                    "The HTTP::username command parses and base64 decodes the username.\n"
                    "The HTTP::password command parses and base64 decodes the password."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n  set auth_sid [AUTH::start pam default_radius]\n}"
                ),
                return_value="Returns the username part of HTTP basic authentication",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::username",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_HEADER,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
