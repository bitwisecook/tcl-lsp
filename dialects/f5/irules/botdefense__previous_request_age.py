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
"""BOTDEFENSE::previous_request_age -- Returns the number of seconds that passed since the previous request was received."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__previous_request_age.html"


@register
class BotdefensePreviousRequestAgeCommand(CommandDef):
    name = "BOTDEFENSE::previous_request_age"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::previous_request_age",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the number of seconds that passed since the previous request was received.",
                synopsis=("BOTDEFENSE::previous_request_age",),
                snippet='Returns the number of seconds that passed since the previous request was received; this is applicable if the current request is a follow-up to a challenge. Otherwise, "0" is returned',
                source=_SOURCE,
                examples=(
                    "# EXAMPLE: Log the previous request age.\n"
                    "when BOTDEFENSE_REQUEST {\n"
                    "    if {[BOTDEFENSE::previous_request_age] != 0} {\n"
                    '        set log "botdefense previous request age is"\n'
                    '        append log " [BOTDEFENSE::previous_request_age]"\n'
                    "        HSL::send $hsl $log\n"
                    "    }\n"
                    "}"
                ),
                return_value="Returns the number of seconds that passed since the previous request was received or 0 if not applicable.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::previous_request_age",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"BOTDEFENSE"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
