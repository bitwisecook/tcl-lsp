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
"""ANTIFRAUD::alert_origin -- Returns or sets the origin of the alert, e.g."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_origin.html"


@register
class AntifraudAlertOriginCommand(CommandDef):
    name = "ANTIFRAUD::alert_origin"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_origin",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets the origin of the alert, e.g.",
                synopsis=("ANTIFRAUD::alert_origin (VALUE)?",),
                snippet=(
                    "ANTIFRAUD::alert_origin ;\n"
                    "                Returns the origin of the alert, e.g. clientside, serverside or secure alert cookie.\n"
                    "\n"
                    "            ANTIFRAUD::alert_origin VALUE ;\n"
                    "                Sets the origin of the alert, e.g. clientside, serverside or secure alert cookie."
                ),
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                log local0. "original Alert origin: [ANTIFRAUD::alert_origin]."\n'
                    "                ANTIFRAUD::alert_origin new_value\n"
                    '                log local0. "new Alert origin: [ANTIFRAUD::alert_origin]."\n'
                    "            }"
                ),
                return_value="ANTIFRAUD::alert_origin ; Returns the origin of the alert, e.g. clientside, serverside or secure alert cookie.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_origin (VALUE)?",
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
