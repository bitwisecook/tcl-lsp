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
"""TCP::lossfilterrate -- Gets the TCP Loss Ignore Rate Parameter."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__lossfilterrate.html"


@register
class TcpLossfilterrateCommand(CommandDef):
    name = "TCP::lossfilterrate"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::lossfilterrate",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets the TCP Loss Ignore Rate Parameter.",
                synopsis=("TCP::lossfilterrate",),
                snippet=(
                    "Gets the maximum number of packets per million lost before triggering congestion response.\n"
                    "  * Rate range is valid from 0 to 1,000,000. Rate is X packets lost per\n"
                    "    million before congestion control kicks in."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "    # Remove loss filter if present\n"
                    "    if { [TCP::lossfilterrate] > 0 } {\n"
                    "        TCP::lossfilter 0 0\n"
                    "    }\n"
                    "}"
                ),
                return_value="TCP Loss Ignore Rate in packets per million.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::lossfilterrate",
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
