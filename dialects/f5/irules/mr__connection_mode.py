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
"""MR::connection_mode -- Returns the connection mode."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__connection_mode.html"


@register
class MrConnectionModeCommand(CommandDef):
    name = "MR::connection_mode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::connection_mode",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the connection mode.",
                synopsis=("MR::connection_mode",),
                snippet=(
                    "returns the connection mode of the current connection and the number of\n"
                    "as configured in the peer object used to create the connection. Valid\n"
                    'connection modes as "per-peer", "per-blade", "per-tmm" or "per-client".\n'
                    'For incoming connections, it will return "per-peer".'
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '    log local0. "[MR::connection_instance] [MR::connection_mode]"\n'
                    "}"
                ),
                return_value="returns the connection mode",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::connection_mode",
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
