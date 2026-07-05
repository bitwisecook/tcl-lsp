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
"""HTTP::payload -- Queries for or manipulates HTTP payload information."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__payload.html"


@register
class HttpPayloadCommand(CommandDef):
    name = "HTTP::payload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::payload",
            dialects=_IRULES_ONLY,
            byte_array_payload=True,
            pure=True,
            hover=HoverSnippet(
                summary="Queries for or manipulates HTTP payload information.",
                synopsis=(
                    "HTTP::payload ( LENGTH | (OFFSET LENGTH) )?",
                    "HTTP::payload length",
                    "HTTP::payload rechunk",
                    "HTTP::payload unchunk",
                ),
                snippet=(
                    "Queries for or manipulates HTTP payload (content) information. With\n"
                    "this command, you can retrieve content, query for content size, or\n"
                    "replace a certain amount of content. The content does not include the\n"
                    "HTTP headers."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE_DATA {\nHTTP::respond 200 content [HTTP::payload]\n}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::payload ( LENGTH | (OFFSET LENGTH) )?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            cse_candidate=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_BODY,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                    scope=StorageScope.EVENT,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
