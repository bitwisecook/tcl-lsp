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
"""BOTDEFENSE::action -- Returns or overrides the action to be taken by Bot Defense."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__action.html"


@register
class BotdefenseActionCommand(CommandDef):
    name = "BOTDEFENSE::action"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::action",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or overrides the action to be taken by Bot Defense.",
                synopsis=("BOTDEFENSE::action (allow |",),
                snippet=(
                    "Returns or overrides the action to be taken by Bot Defense.\n"
                    "\n"
                    'Overriding the action may fail on certain cases. For example, overriding to the "browser_challenge" action, may only be done on requests to which the value of BOTDEFENSE::cs_possible is "true". When overriding the action, the command returns "ok" if the action was successfully set. Otherwise, the action is not changed, and the reason for failure is returned.\n'
                    "\n"
                    'After a successful action override (resulting in the "ok" string), the action cannot be overridden again.'
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE_RELEASE {\n"
                    "    if {[info exists botdefense_responded]} {\n"
                    '        HTTP::header insert "myheader" "blocked request"\n'
                    "    }\n"
                    "}"
                ),
                return_value="* When called without any arguments: Returns a string signifying the action to be taken by Bot Defense. If the action was overridden, the returned action is the overridden one. * When called with an argument for overriding the action, the return value is a status string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::action (allow |",
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
