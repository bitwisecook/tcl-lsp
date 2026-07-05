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
"""BOTDEFENSE::cs_attribute -- Queries for or sets attributes for the client-side challenge."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__cs_attribute.html"


_av = make_av(_SOURCE)


@register
class BotdefenseCsAttributeCommand(CommandDef):
    name = "BOTDEFENSE::cs_attribute"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::cs_attribute",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Queries for or sets attributes for the client-side challenge.",
                synopsis=("BOTDEFENSE::cs_attribute 'device_id' (BOOLEAN)?",),
                snippet="Queries for or sets attributes for the client-side challenge. These attributes are only effective if a client-side action is taken on the current request.",
                source=_SOURCE,
                examples=(
                    "# EXAMPLE: Make sure that the data for the device_id is always collected when taking a client-side action.\n"
                    "when BOTDEFENSE_REQUEST {\n"
                    "    BOTDEFENSE::cs_attribute device_id enable\n"
                    "}"
                ),
                return_value="* When called with an argument the command overrides the decision of Bot Defense whether to collect device id. * When called without an argument, the command returns whether Bot Defense attempts to collect the device id during the request (initiate response).",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::cs_attribute 'device_id' (BOOLEAN)?",
                    arg_values={
                        0: (
                            _av(
                                "device_id",
                                "BOTDEFENSE::cs_attribute device_id",
                                "BOTDEFENSE::cs_attribute 'device_id' (BOOLEAN)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"BOTDEFENSE"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
