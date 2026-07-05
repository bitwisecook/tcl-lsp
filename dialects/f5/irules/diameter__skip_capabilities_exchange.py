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
"""DIAMETER::skip_capabilities_exchange -- Instructs DIAMETER protocol to skip capabilities exchange when establishing a peering relationship."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__skip_capabilities_exchange.html"


@register
class DiameterSkipCapabilitiesExchangeCommand(CommandDef):
    name = "DIAMETER::skip_capabilities_exchange"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::skip_capabilities_exchange",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Instructs DIAMETER protocol to skip capabilities exchange when establishing a peering relationship.",
                synopsis=("DIAMETER::skip_capabilities_exchange ( HOSTNAME )?",),
                snippet=(
                    "Once called, the current connection will skip DIAMETER capabilities exchange message communication with the peer device and will immediately be able to receive DIAMETER messaegs.\n"
                    "\n"
                    "If the HOSTNAME parameter is provided, the provided name will be used as the peer device's origin-host attribute for logging."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '                if { ([IP::address] starts_with "192.168.") } {\n'
                    "                    DIAMETER::skip_capabilities_exchange [IP::address].somesp.com\n"
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::skip_capabilities_exchange ( HOSTNAME )?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(also_in=frozenset({"CLIENT_ACCEPTED"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
