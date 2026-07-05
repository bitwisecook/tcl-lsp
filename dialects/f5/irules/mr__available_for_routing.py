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
"""MR::available_for_routing -- Gets or sets the available_for_routing mode for the current connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__available_for_routing.html"


@register
class MrAvailableForRoutingCommand(CommandDef):
    name = "MR::available_for_routing"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::available_for_routing",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the available_for_routing mode for the current connection.",
                synopsis=("MR::available_for_routing (BOOLEAN)?",),
                snippet="The MR::available_for_routing command sets or resets the available_for_routing mode of the current connection. If available_for_routing mode is enabled (upon completion of CLIENT_ACCEPTED event), the connection will be stored in the internal table of existing connections used for routing messages. This will make the connection available to have request messages routed towards it. If available_for_routing mode is disabled (upon completion of CLIENT_ACCEPTED event), the current connection will not be added to the internal table of existing connections.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "                MR::available_for_routing no\n"
                    "            }"
                ),
                return_value="Returns the current value of the available_for_routing flag. This will be 'true' or 'false'.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::available_for_routing (BOOLEAN)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
