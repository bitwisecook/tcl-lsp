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
"""MR::prime -- establishes an outgoing connection to the specified host or hosts using the specified transport"""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__prime.html"


_av = make_av(_SOURCE)


@register
class MrPrimeCommand(CommandDef):
    name = "MR::prime"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::prime",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="establishes an outgoing connection to the specified host or hosts using the specified transport",
                synopsis=(
                    "MR::prime (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG)) ((pool POOL_OBJ) | (host HOST)))?",
                ),
                snippet="The MR::prime command instructs the Message Routing Framework to establish an outgoing connection to a specified host or pool if one does not exist. The setting of the specified virtual or transport-config will be used to establish the connection. If a pool is provided, outgoing connections will be created to all active poolmembers of the specified pool.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "                MR::prime config /Common/my_tc pool /Common/default_pool\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::prime (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG)) ((pool POOL_OBJ) | (host HOST)))?",
                    arg_values={
                        0: (
                            _av(
                                "virtual",
                                "MR::prime virtual",
                                "MR::prime (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG)) ((pool POOL_OBJ) | (host HOST)))?",
                            ),
                            _av(
                                "config",
                                "MR::prime config",
                                "MR::prime (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG)) ((pool POOL_OBJ) | (host HOST)))?",
                            ),
                            _av(
                                "pool",
                                "MR::prime pool",
                                "MR::prime (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG)) ((pool POOL_OBJ) | (host HOST)))?",
                            ),
                            _av(
                                "host",
                                "MR::prime host",
                                "MR::prime (((virtual VIRTUAL_SERVER_OBJ) | (config TRANSPORT_CONFIG)) ((pool POOL_OBJ) | (host HOST)))?",
                            ),
                        )
                    },
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
