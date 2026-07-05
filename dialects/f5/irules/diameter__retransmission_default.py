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
"""DIAMETER::retransmission_default -- Gets of sets the current connection's retransmission settings."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__retransmission_default.html"


@register
class DiameterRetransmissionDefaultCommand(CommandDef):
    name = "DIAMETER::retransmission_default"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::retransmission_default",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets of sets the current connection's retransmission settings.",
                synopsis=("DIAMETER::retransmission_default action",),
                snippet=(
                    "This command allows the setting or getting of the current\n"
                    "connection\\'s retransmission settings. All request messages on the\n"
                    "current connection will be initailized with the connection\\'s setings.\n"
                    "The messages\\'s settings may be changed with the\n"
                    "DIAMETER::retransmission command.\n"
                    "        \n"
                    "Gets the current connection\\'s retransmission action.\n"
                    "Possible actions are:\n"
                    "\n"
                    ' * "disabled" - request messages will not be queued for retransmission\n'
                    "\n"
                    ' * "busy" - when retransmission is triggered for a request message an\n'
                    "   answer message with a DIAMETER_TOO_BUSY result code will be\n"
                    "   returned to the originator."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n    DIAMETER::retransmission_default action busy\n}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::retransmission_default action",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                also_in=frozenset({"CLIENT_ACCEPTED", "SERVER_CONNECTED"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
