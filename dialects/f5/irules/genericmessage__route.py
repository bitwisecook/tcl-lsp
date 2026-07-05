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
"""GENERICMESSAGE::route -- Adds, deletes, or looks up message routes."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/GENERICMESSAGE__route.html"


_av = make_av(_SOURCE)


@register
class GenericmessageRouteCommand(CommandDef):
    name = "GENERICMESSAGE::route"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GENERICMESSAGE::route",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Adds, deletes, or looks up message routes.",
                synopsis=(
                    "GENERICMESSAGE::route (add | delete | lookup) ((('virtual' VIRTUAL_SERVER_OBJ)",
                ),
                snippet=(
                    "The GENERICMESSAGE::route command allows you to add, delete, or lookup\n"
                    "message routes."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '    GENERICMESSAGE::route add dst "client-[IP::remote_addr]" host "[IP::remote_addr]:[TCP::remote_port]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GENERICMESSAGE::route (add | delete | lookup) ((('virtual' VIRTUAL_SERVER_OBJ)",
                    arg_values={
                        0: (
                            _av(
                                "virtual",
                                "GENERICMESSAGE::route virtual",
                                "GENERICMESSAGE::route (add | delete | lookup) ((('virtual' VIRTUAL_SERVER_OBJ)",
                            ),
                            _av(
                                "add",
                                "GENERICMESSAGE::route add",
                                "GENERICMESSAGE::route (add | delete | lookup) ((('virtual' VIRTUAL_SERVER_OBJ)",
                            ),
                            _av(
                                "delete",
                                "GENERICMESSAGE::route delete",
                                "GENERICMESSAGE::route (add | delete | lookup) ((('virtual' VIRTUAL_SERVER_OBJ)",
                            ),
                            _av(
                                "lookup",
                                "GENERICMESSAGE::route lookup",
                                "GENERICMESSAGE::route (add | delete | lookup) ((('virtual' VIRTUAL_SERVER_OBJ)",
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
