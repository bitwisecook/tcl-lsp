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
"""findstr -- Finds a string within another string and returns the string starting at the offset specified from the match."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/findstr.html"


@register
class FindstrCommand(CommandDef):
    name = "findstr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="findstr",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Finds a string within another string and returns the string starting at the offset specified from the match.",
                synopsis=("findstr STRING SEARCH_STRING (",),
                snippet=(
                    "A custom iRule function which finds a string within another string\n"
                    "and returns the string starting at the offset specified from the match."
                ),
                source=_SOURCE,
                examples=(
                    "when RULE_INIT {\n"
                    '  set static::payload {<meta HTTP-EQUIV="REFRESH" CONTENT="0; URL=https://host.domain.com/path/file.ext?...&var=val">}\n'
                    '  set static::term {">}\n'
                    "  set urlresponse [findstr $static::payload URL= 4 $static::term]\n"
                    '  log local0. "urlresponse $urlresponse"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="findstr STRING SEARCH_STRING (",
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
