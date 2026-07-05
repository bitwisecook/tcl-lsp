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
"""ISTATS::incr -- Increments the specified key by the given value."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ISTATS__incr.html"


@register
class IstatsIncrCommand(CommandDef):
    name = "ISTATS::incr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ISTATS::incr",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Increments the specified key by the given value.",
                synopsis=("ISTATS::incr KEY VALUE",),
                snippet=(
                    "Increments the specified key by the given value. The increment value must be non-negative for a counter.\n"
                    "\n"
                    "Note that text string iStats may not be incremented."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '        if { [string tolower [HTTP::uri]] equals "/12345" } {\n'
                    '                ISTATS::incr "uri /12345 counter Requests" 1\n'
                    '                HTTP::uri "/"\n'
                    '                HTTP::redirect "http://www.mysite.com"\n'
                    '        } elseif { [string tolower [HTTP::uri]] equals "/stats" } {\n'
                    '                  HTTP::respond 200 content "<html><body>Requests for /12345: [ISTATS::get "uri /12345 counter Requests"]</body></html>"\n'
                    "        }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ISTATS::incr KEY VALUE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ISTATS,
                    writes=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
