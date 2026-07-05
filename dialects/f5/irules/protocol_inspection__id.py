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
"""PROTOCOL_INSPECTION::id -- Provides protocol inspection match result."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PROTOCOL_INSPECTION__id.html"


@register
class ProtocolInspectionIdCommand(CommandDef):
    name = "PROTOCOL_INSPECTION::id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PROTOCOL_INSPECTION::id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Provides protocol inspection match result.",
                synopsis=("PROTOCOL_INSPECTION::id",),
                snippet="This command provides inspection match result.",
                source=_SOURCE,
                examples=(
                    "when PROTOCOL_INSPECTION_MATCH {\n"
                    "    set id [PROTOCOL_INSPECTION::id]\n"
                    '    log local0.debug "inspection id: $id"\n'
                    "}"
                ),
                return_value="PROTOCOL_INSPECTION::id returns inspection id array",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PROTOCOL_INSPECTION::id",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"PROTOCOL_INSPECTION"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
