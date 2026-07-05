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
"""HTTP::is_keepalive -- Returns a true value if this is a Keep-Alive connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__is_keepalive.html"


@register
class HttpIsKeepaliveCommand(CommandDef):
    name = "HTTP::is_keepalive"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::is_keepalive",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a true value if this is a Keep-Alive connection.",
                synopsis=("HTTP::is_keepalive",),
                snippet="Returns a true value if this is a Keep-Alive connection.",
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n  if {[HTTP::is_keepalive]}{\n    HTTP::close\n  }\n}"
                ),
            ),
            pure=True,
            forms=(
                FormSpec(
                    kind=FormKind.GETTER,
                    synopsis="HTTP::is_keepalive",
                    arity=Arity(0, 0),
                    pure=True,
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0, 0),
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
