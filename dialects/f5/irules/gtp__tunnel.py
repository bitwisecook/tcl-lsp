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
"""GTP::tunnel -- These commands parse the payload of G-PDU as IP datagram and return the values from IP header and TCP/UDP header."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/GTP__tunnel.html"


_av = make_av(_SOURCE)


@register
class GtpTunnelCommand(CommandDef):
    name = "GTP::tunnel"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GTP::tunnel",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="These commands parse the payload of G-PDU as IP datagram and return the values from IP header and TCP/UDP header.",
                synopsis=("GTP::tunnel ('is_ip'",),
                snippet=(
                    "These commands parse the payload of G-PDU as IP datagram and return the\n"
                    "values from IP header and TCP/UDP header.\n"
                    "When parsed payload contains a value other than 4 or 6 for IP version,\n"
                    'the commands return an empty value. "is_ip" can be used to confirm if\n'
                    "parser is considering the payload as ip-datagram or not. The commands\n"
                    "return empty for non G-PDU messages.\n"
                    "tcp_ and udp_ commands return empty value if the ip-proto in the\n"
                    'ip-datagram does not match. "GTP::tunnel ip_proto" may be used to\n'
                    "verify before calling transport level commands."
                ),
                source=_SOURCE,
                examples=(
                    "when GTP_SIGNALLING_INGRESS {\n"
                    '    log local0. "GTP tunnel TCP src port [GTP::tunnel tcp_src_port]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GTP::tunnel <subcommand> ?-message MESSAGE?",
                    options=(
                        OptionSpec(
                            name="-message",
                            detail="Operate on a specific GTP message object.",
                            takes_value=True,
                            value_hint="MESSAGE",
                        ),
                    ),
                    arg_values={0: (_av("is_ip", "GTP::tunnel is_ip", "GTP::tunnel ('is_ip'"),)},
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
