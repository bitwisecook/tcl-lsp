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
"""LINK::qos -- F5 iRules command `LINK::qos`."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LINK__qos.html"


@register
class LinkQosCommand(CommandDef):
    name = "LINK::qos"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LINK::qos",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the QoS level set for the current packet.",
                synopsis=("LINK::qos",),
                snippet=(
                    "Returns the QoS level set for the current packet.\n"
                    "The Quality of Service (QoS) standard is a means by which network\n"
                    "equipment can identify and treat traffic differently based on an\n"
                    "identifier.\n"
                    "This command can be used to direct traffic based on the QoS level\n"
                    "within a packet.\n"
                    "This command is equivalent to the BIG-IP 4.X variable link_qos."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "  if { [LINK::qos] > 2 } {\n"
                    "     pool fast_pool\n"
                    "  } else {\n"
                    "     pool slow_pool\n"
                    " }\n"
                    "}"
                ),
                return_value="LINK::qos",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LINK::qos",
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
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
