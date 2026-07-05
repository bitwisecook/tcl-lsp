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
"""STATS::incr -- Increments the value of a Statistics profile setting."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/STATS__incr.html"


@register
class StatsIncrCommand(CommandDef):
    name = "STATS::incr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="STATS::incr",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Increments the value of a Statistics profile setting.",
                synopsis=("STATS::incr PROFILE_NAME FIELD_NAME (VALUE)?",),
                snippet=(
                    "Increments the value of the specified setting (field), in the specified\n"
                    "Statistics profile, by the specified value. If you do not specify a\n"
                    "value, the system increments by 1. It is possible to set a negative\n"
                    "value in order to decrement the counter. Returns the current value of\n"
                    "the field which was incremented."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "\n"
                    "   # Increment the number of unanswered HTTP requests\n"
                    '   log local0. "Incremented the current count to: [STATS::incr my_stats_profile_name "current_count"]"\n'
                    "}"
                ),
                return_value="Returns the current value of the field which was incremented.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="STATS::incr PROFILE_NAME FIELD_NAME (VALUE)?",
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
