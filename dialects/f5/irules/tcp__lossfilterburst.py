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
"""TCP::lossfilterburst -- Gets the TCP Loss Ignore Burst Parameter."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__lossfilterburst.html"


@register
class TcpLossfilterburstCommand(CommandDef):
    name = "TCP::lossfilterburst"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::lossfilterburst",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets the TCP Loss Ignore Burst Parameter.",
                synopsis=("TCP::lossfilterburst",),
                snippet=(
                    "Gets the maximum size burst loss (in packets) before triggering congestion response.\n"
                    "  * Burst range is valid from 0 to 32. Higher values decrease the\n"
                    "    chance of performing congestion control."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "    # Set loss filter burst to a maximum of 3\n"
                    "    if { [TCP::lossfilterburst] > 3 } {\n"
                    "        TCP::lossfilter [TCP::lossfilterrate] 3\n"
                    "    }\n"
                    "}"
                ),
                return_value="TCP Loss Ignore Burst in packets.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::lossfilterburst",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
