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
"""virtual -- Returns the name of the associated virtual server or selects another virtual server and an optional IP address and port to connect to."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/virtual.html"


_av = make_av(_SOURCE)


@register
class VirtualCommand(CommandDef):
    name = "virtual"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="virtual",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the name of the associated virtual server or selects another virtual server and an optional IP address and port to connect to.",
                synopsis=(
                    "virtual",
                    "virtual (name | VIRTUAL_SERVER_OBJ) (IP_TUPLE | (IP_ADDR (PORT)?))?",
                ),
                snippet=(
                    "Returns the name of the associated virtual server that the connection\n"
                    "is flowing through. In 9.4.0 and higher, it can be also used to route\n"
                    "the connection to another virtual server and an optional IP address\n"
                    "and port, without leaving the BIG-IP."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '  log local0. "Current virtual server name: [virtual name]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="virtual",
                    arg_values={
                        0: (
                            _av(
                                "name",
                                "virtual name",
                                "virtual (name | VIRTUAL_SERVER_OBJ) (IP_TUPLE | (IP_ADDR (PORT)?))?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            cse_candidate=True,
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
