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
"""DIAMETER::state -- Returns the current state of the Diameter peer's connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__state.html"


@register
class DiameterStateCommand(CommandDef):
    name = "DIAMETER::state"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::state",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the current state of the Diameter peer's connection.",
                synopsis=("DIAMETER::state",),
                snippet=(
                    "This iRule command returns the current state of the Diameter peer\\'s\n"
                    "connection, as a string. There are five possible states:\n"
                    "  * CLOSED - The connection is down\n"
                    "  * WAIT_ICEA - still waiting for the initial CEA\n"
                    "  * ROPEN - The connection has been reopened\n"
                    "  * IOPEN - The connection is open for the first time\n"
                    "  * CLOSING - The connection will soon be down"
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n"
                    '    if { [DIAMETER::state] == "ROPEN" } {\n'
                    '        log local0. "Received a DIAMETER message via a reopened connection"\n'
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::state",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DIAMETER", "MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
