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
"""DIAMETER::is_retransmission -- Returns true if it is a retransmitted request, otherwise, returns false."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__is_retransmission.html"


@register
class DiameterIsRetransmissionCommand(CommandDef):
    name = "DIAMETER::is_retransmission"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::is_retransmission",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns true if it is a retransmitted request, otherwise, returns false.",
                synopsis=("DIAMETER::is_retransmission",),
                snippet=(
                    "This iRule command returns true if the current message is a retransmitted request.\n"
                    "Otherwise, it returns false."
                ),
                source=_SOURCE,
                examples=(
                    "when MR_INGRESS {\n"
                    "    if { [DIAMETER::is_retransmission] } {\n"
                    "        DIAMETER::persist reset\n"
                    "        MR::message route pool /Common/alt_pool\n"
                    "    }\n"
                    "}"
                ),
                return_value="TRUE or FALSE",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::is_retransmission",
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
