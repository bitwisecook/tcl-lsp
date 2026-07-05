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
"""CACHE::trace -- Dump the list of cached objects for a HTTP profile where RAM Cache is enabled."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CACHE__trace.html"


@register
class CacheTraceCommand(CommandDef):
    name = "CACHE::trace"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CACHE::trace",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Dump the list of cached objects for a HTTP profile where RAM Cache is enabled.",
                synopsis=("CACHE::trace (MAX)?",),
                snippet=(
                    "Dump the list of cached objects for a HTTP profile where RAM Cache is\n"
                    "enabled.\n"
                    "This event will execute only if a RAM Cache profile is enabled on the\n"
                    "Virtual Server, and for objects that match the RAM Cache configuration.\n"
                    "The list will represent the size of the cache (Cache Size), number of\n"
                    "objects (Cache Count), and starting by the term Entity, it will list\n"
                    "every object:\n"
                    "  * Pos (0001), list the position of the object in the cache\n"
                    "  * Local Hits (00031/00007) indicate the number of Local Hits\n"
                    "  * Remote Hits (00031/00007) indicate the number of Remote Hits"
                ),
                source=_SOURCE,
                examples=('when RULE_INIT {\n    set static::cache ""\n}'),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CACHE::trace (MAX)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"CACHE"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
