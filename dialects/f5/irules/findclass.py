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
"""findclass -- Searches a data group list for a member that starts with the specified string and returns the data-group member string."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/findclass.html"


@register
class FindclassCommand(CommandDef):
    name = "findclass"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="findclass",
            deprecated_replacement="class match / class search",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Searches a data group list for a member that starts with the specified string and returns the data-group member string.",
                synopsis=("findclass STRING DATA_GROUP (SEPARATOR)?",),
                snippet=(
                    "Searches a data group list for a member whose key matches the specified\n"
                    "string, and if a match is found, returns the data-group member string.\n"
                    "\n"
                    "Note: findclass has been deprecated in v10 in favor of the new\n"
                    "class commands. The class command offers better functionality and\n"
                    "performance than findclass\n"
                    "Only the key value of the data group list member (the portion up to the\n"
                    "first separator character, which defaults to space unless otherwise\n"
                    "specified) is compared to the specified string to determine a match."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '  set location [findclass [HTTP::uri] URIredirects_dg " "]\n'
                    '  if { $location ne "" } {\n'
                    "    HTTP::redirect $location\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="findclass STRING DATA_GROUP (SEPARATOR)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DATA_GROUP,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
