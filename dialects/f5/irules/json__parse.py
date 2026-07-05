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
"""JSON::parse -- Parses JSON content into a JSON cache that can be manipulated using further JSON:: commands."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/JSON__parse.html"


@register
class JsonParseCommand(CommandDef):
    name = "JSON::parse"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="JSON::parse",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Parses JSON content into a JSON cache that can be manipulated using further JSON:: commands.",
                synopsis=("JSON::parse (JSON_STRING (JSON_MAX_ENTRIES)? )?",),
                snippet=(
                    "If a string is omitted, returns any JSON cache that preexists in the context in which this is executed. This is the normal case when the command is executed in the JSON_REQUEST or JSON_RESPONSE event.\n"
                    "If a string is provided, it is assumed to contain JSON and is parsed into a new JSON cache. This will be deleted when it is no longer referenced by a Tcl variable. This is useful when a JSON profile is not being used."
                ),
                source=_SOURCE,
                examples=("when JSON_REQUEST {\n    JSON::render\n}"),
                return_value="Returns a JSON cache instance handle to use for retrieving and overwriting content, and rendering.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="JSON::parse (JSON_STRING (JSON_MAX_ENTRIES)? )?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
