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
"""CACHE::age -- Returns the age of the document in the cache."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CACHE__age.html"


@register
class CacheAgeCommand(CommandDef):
    name = "CACHE::age"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CACHE::age",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the age of the document in the cache.",
                synopsis=("CACHE::age",),
                snippet=(
                    "Returns the age of the document in the cache, in seconds.\n"
                    "\n"
                    "CACHE::age\n"
                    "\n"
                    "     * Returns the age of the document in the cache, in seconds."
                ),
                source=_SOURCE,
                examples=(
                    "when CACHE_REQUEST {\n"
                    "  if { [CACHE::age] > 60 } {\n"
                    "    CACHE::expire\n"
                    '    log local0. "Expiring content: Age > 60 seconds"\n'
                    "   }\n"
                    "}"
                ),
                return_value="Returns the age of the document in the cache, in seconds.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CACHE::age",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
