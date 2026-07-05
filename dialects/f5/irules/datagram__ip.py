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
"""DATAGRAM::ip -- Returns ip header information."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DATAGRAM__ip.html"


_av = make_av(_SOURCE)


@register
class DatagramIpCommand(CommandDef):
    name = "DATAGRAM::ip"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DATAGRAM::ip",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns ip header information.",
                synopsis=(
                    "DATAGRAM::ip (tos | ttl | flags)",
                    "DATAGRAM::ip (option | option_count) (IPV4_OPTION)?",
                ),
                snippet=(
                    "This iRules command returns ip header information.\n"
                    "\n"
                    "DATAGRAM::ip tos\n"
                    "\n"
                    "     * Returns IP header ToS as an integer value.\n"
                    "\n"
                    "DATAGRAM::ip ttl\n"
                    "\n"
                    "     * Returns IP header TTL as an integer value.\n"
                    "\n"
                    "DATAGRAM::ip flags\n"
                    "\n"
                    "     * Returns IP header flags as an integer value. The flags are from the\n"
                    "       IP datagram after IP fragment reassembly. Any MF flags that were\n"
                    "       present in indivdual fragments will not be returned. DF flag is\n"
                    "       preserved if it was set.\n"
                    "\n"
                    "DATAGRAM::ip option\n"
                    "\n"
                    "     * This command returns a Tcl list of IP options from reassembled IP\n"
                    "       datagram."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DATAGRAM::ip (tos | ttl | flags)",
                    arg_values={
                        0: (
                            _av("tos", "DATAGRAM::ip tos", "DATAGRAM::ip (tos | ttl | flags)"),
                            _av("ttl", "DATAGRAM::ip ttl", "DATAGRAM::ip (tos | ttl | flags)"),
                            _av("flags", "DATAGRAM::ip flags", "DATAGRAM::ip (tos | ttl | flags)"),
                            _av(
                                "option",
                                "DATAGRAM::ip option",
                                "DATAGRAM::ip (option | option_count) (IPV4_OPTION)?",
                            ),
                            _av(
                                "option_count",
                                "DATAGRAM::ip option_count",
                                "DATAGRAM::ip (option | option_count) (IPV4_OPTION)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"FLOW"}), also_in=frozenset({"CLIENT_DATA"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
