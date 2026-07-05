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
"""UDP::debug_queue -- This command can be used to enable/disable printing debug messages when UDP::max_rate iRule is in use."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/UDP__debug_queue.html"


@register
class UdpDebugQueueCommand(CommandDef):
    name = "UDP::debug_queue"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="UDP::debug_queue",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command can be used to enable/disable printing debug messages when UDP::max_rate iRule is in use.",
                synopsis=("UDP::debug_queue BOOL_VALUE",),
                snippet=(
                    "UDP::debug_queue enable starts printing debug messages related to UDP::max_rate.\n"
                    "UDP::debug_queue disable stops printing debug messages related to UDP::max_rate."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "    # Set the rate to 1Mbps (125,000 bytes per second)\n"
                    '    log local0. "UDP set max rate: [UDP::max_rate 125000]"\n'
                    '    log local0. "UDP get max rate: [UDP::max_rate]"\n'
                    "    # Enable printing debug messages.\n"
                    '    log local0. "Enable debugging [UDP::debug_queue enable]"\n'
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="UDP::debug_queue BOOL_VALUE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UDP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
