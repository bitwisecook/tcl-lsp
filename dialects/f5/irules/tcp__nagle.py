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
"""TCP::nagle -- Toggles the Nagle mode."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__nagle.html"


_av = make_av(_SOURCE)


@register
class TcpNagleCommand(CommandDef):
    name = "TCP::nagle"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::nagle",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Toggles the Nagle mode.",
                synopsis=("TCP::nagle (enable | disable | auto)",),
                snippet=(
                    "Enables or disables the Nagle algorithm on the current TCP connection.\n"
                    "Nagle waits for additional data before sending undersized packets, see RFC896 for details.\n"
                    "The auto option enables or disables Nagle based on connection conditions."
                ),
                source=_SOURCE,
                examples=(
                    "# Change the TCP Nagle mode to auto.\n"
                    "when CLIENT_ACCEPTED {\n"
                    "    TCP::nagle auto\n"
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::nagle (enable | disable | auto)",
                    arg_values={
                        0: (
                            _av(
                                "enable",
                                "TCP::nagle enable",
                                "TCP::nagle (enable | disable | auto)",
                            ),
                            _av(
                                "disable",
                                "TCP::nagle disable",
                                "TCP::nagle (enable | disable | auto)",
                            ),
                            _av("auto", "TCP::nagle auto", "TCP::nagle (enable | disable | auto)"),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                transport="tcp",
                also_in=frozenset({"SIP_REQUEST", "SIP_REQUEST_SEND", "SIP_RESPONSE"}),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
