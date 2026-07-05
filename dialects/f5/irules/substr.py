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
"""substr -- Returns a substring from a string."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/substr.html"


@register
class SubstrCommand(CommandDef):
    name = "substr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="substr",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a substring from a string.",
                synopsis=("substr STRING SKIP_COUNT (TERMINATOR)?",),
                snippet=(
                    "A custom iRule function which returns a substring named <string>,\n"
                    "based on the values of the <skip_count> and <terminator> arguments.\n"
                    "Note the following:\n"
                    "  * The <skip_count> and <terminator> arguments are used in the same\n"
                    "    way as they are for the findstr command.\n"
                    "  * The <skip_count> argument is the index into <string> of the first\n"
                    "    character to be returned, where 0 indicates the first character of\n"
                    "    <string>.\n"
                    "  * The <terminator> argument can be either the subtring length or the\n"
                    "    substring terminating string."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '  set uri [substr $uri 1 "?"]\n'
                    '  log local0. "Uri Part = $uri"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="substr STRING SKIP_COUNT (TERMINATOR)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
