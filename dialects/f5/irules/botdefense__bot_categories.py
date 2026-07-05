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
"""BOTDEFENSE::bot_categories -- Returns the list of category names to which the current client belongs."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__bot_categories.html"


@register
class BotdefenseBotCategoriesCommand(CommandDef):
    name = "BOTDEFENSE::bot_categories"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::bot_categories",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the list of category names to which the current client belongs.",
                synopsis=("BOTDEFENSE::bot_categories",),
                snippet="Returns the list of category names to which the current client belongs. These categories are determined by the anomalies found for the respective client. Note these categories are additional to the bot signature category which is applicable if a bot signature was found.",
                source=_SOURCE,
                examples=(
                    "when BOTDEFENSE_ACTION {\n"
                    "    foreach {cat} [BOTDEFENSE::bot_categories] {\n"
                    '        log.local0. "Found category: $cat"\n'
                    "    }\n"
                    "}"
                ),
                return_value="Returns a list of all category names to which the current client belongs based on the anomalies found for the client. The categories come in addition to the bot signature category optionally detected and returned in BOTDEFENSE::bot_signature_category. If no anomaly found then the list will be empty.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::bot_categories",
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
