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
"""HTTP::has_responded -- Returns true if this HTTP transaction has been prematurely completed by an iRule command or other filter logic."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__has_responded.html"


@register
class HttpHasRespondedCommand(CommandDef):
    name = "HTTP::has_responded"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::has_responded",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns true if this HTTP transaction has been prematurely completed by an iRule command or other filter logic.",
                synopsis=("HTTP::has_responded",),
                snippet="This can be triggered by HTTP::respond, HTTP::redirect, HTTP::retry, and some ACCESS commands.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  # Used for cases where only one response to the client is permitted.\n"
                    "  # Another HTTP::respond might have been called in other iRULE script.\n"
                    "  if {[HTTP::has_responded]} {\n"
                    '    log local0. "Have already responded."\n'
                    "  } else {\n"
                    "    HTTP::respond 200 content {<html><body>First and Only Response</body></html>}\n"
                    "  }\n"
                    "}"
                ),
            ),
            pure=True,
            forms=(
                FormSpec(
                    kind=FormKind.GETTER,
                    synopsis="HTTP::has_responded",
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
                    target=SideEffectTarget.RESPONSE_COMMIT,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
