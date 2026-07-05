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
"""MR::always_match_port -- Gets or sets the always_match_port mode for the router."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__always_match_port.html"


@register
class MrAlwaysMatchPortCommand(CommandDef):
    name = "MR::always_match_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::always_match_port",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the always_match_port mode for the router.",
                synopsis=("MR::always_match_port (BOOLEAN)?",),
                snippet="The MR::always_match_port command sets or resets the always_match_port mode of the current router. If always_match_port mode is enabled (upon completion of CLIENT_ACCEPTED event), the router will only forward messages to existing connections where the remote port matches the remote port of the selected destination. If an existing connection is not found, a new connection will be created. Setting this mode will keep MRF from forwarding messages to incoming connections (since the incoming connection likely uses a ephemeral port as the source port).",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "                MR::always_match_port no\n"
                    "            }"
                ),
                return_value="Returns the current value of the always_match_port flag. This will be 'true' or 'false'.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::always_match_port (BOOLEAN)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
