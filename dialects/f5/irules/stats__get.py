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
"""STATS::get -- Retrieves a setting value from a Statistics profile."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/STATS__get.html"


@register
class StatsGetCommand(CommandDef):
    name = "STATS::get"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="STATS::get",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Retrieves a setting value from a Statistics profile.",
                synopsis=("STATS::get PROFILE_NAME FIELD_NAME",),
                snippet=(
                    "Retrieves the value of the specified field of the specified Statistics\n"
                    "profile.\n"
                    "\n"
                    "STATS::get <profile> <field>\n"
                    "\n"
                    "     * Retrieves the value of the specified field of the specified\n"
                    "       Statistics profile."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '  if {[string tolower [HTTP::uri]] starts_with "/check"} {\n'
                    '    STATS::get stats_profile_1 "my_first_field"\n'
                    "  }\n"
                    "}"
                ),
                return_value="Returns the value of the specified field of the specified Statistics profile",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="STATS::get PROFILE_NAME FIELD_NAME",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ISTATS,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
