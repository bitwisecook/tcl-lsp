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
"""CLASSIFY::urlcat -- Allows to set or add an url category to the classification."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CLASSIFY__urlcat.html"


_av = make_av(_SOURCE)


@register
class ClassifyUrlcatCommand(CommandDef):
    name = "CLASSIFY::urlcat"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CLASSIFY::urlcat",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Allows to set or add an url category to the classification.",
                synopsis=("CLASSIFY::urlcat ('set' | 'add') CLASSIFY_URL_CATEGORY_NAME",),
                snippet=(
                    "This command allows you to set or add an url category to the\n"
                    "classification.\n"
                    "\n"
                    "* Note: APM / AFM / PEM license is required for functionality to work.\n"
                    "\n"
                    "CLASSIFY::urlcat set <URL_Category>\n"
                    "\n"
                    "     * will immediately classify flow as URL_category.\n"
                    "\n"
                    "CLASSIFY::application add <app_name>\n"
                    "\n"
                    "     * adds an URL Category to the URL classification token to the final\n"
                    "       classification result issued by the classification engine. This can\n"
                    "       be issued multiple times in order to add multiple tokens to the classification result."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST\n"
                    "{\n"
                    '    if { [HTTP::host] contains "google"} {\n'
                    "        CLASSIFY::urlcat set customCategory\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CLASSIFY::urlcat ('set' | 'add') CLASSIFY_URL_CATEGORY_NAME",
                    arg_values={
                        0: (
                            _av(
                                "set",
                                "CLASSIFY::urlcat set",
                                "CLASSIFY::urlcat ('set' | 'add') CLASSIFY_URL_CATEGORY_NAME",
                            ),
                            _av(
                                "add",
                                "CLASSIFY::urlcat add",
                                "CLASSIFY::urlcat ('set' | 'add') CLASSIFY_URL_CATEGORY_NAME",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CLASSIFICATION_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
