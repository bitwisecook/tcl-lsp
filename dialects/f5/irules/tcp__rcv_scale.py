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
"""TCP::rcv_scale -- Returns the receive window scale advertised by the remote host."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__rcv_scale.html"


@register
class TcpRcvScaleCommand(CommandDef):
    name = "TCP::rcv_scale"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::rcv_scale",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the receive window scale advertised by the remote host.",
                synopsis=("TCP::rcv_scale",),
                snippet="Returns the receive window scale advertised by the remote host.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    # Log rcv_scale.\n"
                    '    log local0. "rcv_scale: [TCP::rcv_scale]"\n'
                    "}"
                ),
                return_value="The bitshift associated with the remote host window scale.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::rcv_scale",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            excluded_events=("SERVER_INIT",),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
