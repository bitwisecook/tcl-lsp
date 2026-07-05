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
"""CACHE::disabled -- Returns state of cache disable flag"""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CACHE__disabled.html"


@register
class CacheDisabledCommand(CommandDef):
    name = "CACHE::disabled"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CACHE::disabled",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns state of cache disable flag",
                synopsis=("CACHE::disabled",),
                snippet=(
                    "Returns 1 for cache disabled, 0 otherwise.\n"
                    "\n"
                    "CACHE::disabled\n"
                    "\n"
                    "     * Returns true (1) or false (0) to indicate state of cache disable flag."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    "    set val [CACHE::disabled]\n"
                    '    log local0. "Cache disable state: $val"\n'
                    "}"
                ),
                return_value="Returns 0 or 1.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CACHE::disabled",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
