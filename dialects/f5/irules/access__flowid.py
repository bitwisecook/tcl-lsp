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
"""ACCESS::flowid -- Sets the flow id for SSL Orchestrator using APM logging framework."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ACCESS__flowid.html"


@register
class AccessFlowidCommand(CommandDef):
    name = "ACCESS::flowid"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ACCESS::flowid",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the flow id for SSL Orchestrator using APM logging framework.",
                synopsis=("ACCESS::flowid (FID)?",),
                snippet=(
                    "ACCESS::flowid [FID]\n"
                    "\n"
                    "Calculates the flow id from the IFC and 4-tuple information, if it doesn't\n"
                    "exist already, and stores it in the opaque storage for the connflow.\n"
                    "Requires APM to be provisioned.\n"
                    "\n"
                    "Command Syntax\n"
                    "\n"
                    "ACCESS::flowid\n"
                    "\n"
                    "    * Returns the flow id, if it exists, or calculates it, then stores it in\n"
                    "      the opaque data structure for the connflow.\n"
                    "\n"
                    "ACCESS::flowid <FID>\n"
                    "\n"
                    "    * Sets the flow id to FID"
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    ACCESS::flowid "example"\n'
                    "    set ctx(FID) [ACCESS::flowid]\n"
                    "}"
                ),
                return_value="The flow id is returned",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ACCESS::flowid (FID)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
