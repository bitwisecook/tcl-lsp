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
"""BOTDEFENSE::cs_possible -- Returns whether it is possible for Bot Defense to take a client-side action."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__cs_possible.html"


@register
class BotdefenseCsPossibleCommand(CommandDef):
    name = "BOTDEFENSE::cs_possible"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::cs_possible",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns whether it is possible for Bot Defense to take a client-side action.",
                synopsis=("BOTDEFENSE::cs_possible",),
                snippet=(
                    'Returns "true" or "false" based on whether it is possible to take one of the client-side actions that initiate a response (browser challenge, or CAPTCHA challenge, or device id collection) or send browser challenge in response. Certain characteristics of a request make it impossible to respond with a browser verification or CAPTCHA challenge or device id, in which case "false" is returned.\n'
                    "\n"
                    'Setting to a client-side action with BOTDEFENSE::action, while the value of BOTDEFENSE::cs_possible is "false", will fail.'
                ),
                source=_SOURCE,
                examples=(
                    "# EXAMPLE: Prevent blocking of requests that cannot be responded with a\n"
                    "# client-side challenge.\n"
                    "when BOTDEFENSE_ACTION {\n"
                    '    if {    ([BOTDEFENSE::action] eq "tcp_rst") &&\n'
                    "            (not [BOTDEFENSE::cs_possible])} {\n"
                    "        BOTDEFENSE::action allow\n"
                    "    }\n"
                    "}"
                ),
                return_value="Returns a boolean value (0 or 1), whether taking a client-side action is possible.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::cs_possible",
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
