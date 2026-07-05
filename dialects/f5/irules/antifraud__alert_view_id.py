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
"""ANTIFRAUD::alert_view_id -- Returns or sets the configured URL and view which triggered this alert."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_view_id.html"


@register
class AntifraudAlertViewIdCommand(CommandDef):
    name = "ANTIFRAUD::alert_view_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_view_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets the configured URL and view which triggered this alert.",
                synopsis=("ANTIFRAUD::alert_view_id (VALUE)?",),
                snippet=(
                    "ANTIFRAUD::alert_view_id ;\n"
                    "                Returns the configured URL and view which triggered this alert. Empty if not a view.\n"
                    "\n"
                    "            ANTIFRAUD::alert_view_id VALUE ;\n"
                    "                Sets the configured URL and view which triggered this alert. Empty if not a view."
                ),
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                log local0. "original Alert View ID: [ANTIFRAUD::alert_view_id]."\n'
                    "                ANTIFRAUD::alert_view_id new_value\n"
                    '                log local0. "new Alert View ID: [ANTIFRAUD::alert_view_id]."\n'
                    "            }"
                ),
                return_value="ANTIFRAUD::alert_view_id ; Returns the configured URL and view which triggered this alert. Empty if not a view.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_view_id (VALUE)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ANTIFRAUD"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
