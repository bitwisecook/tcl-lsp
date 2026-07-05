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
"""MR::payload -- Access data collected using MR::collect command."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__payload.html"


_av = make_av(_SOURCE)


@register
class MrPayloadCommand(CommandDef):
    name = "MR::payload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::payload",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Access data collected using MR::collect command.",
                synopsis=("MR::payload ( 'length' )?",),
                snippet=(
                    "This command can be used to access payload collected using the COLLECT command.\n"
                    "\n"
                    "SYNTAX\n"
                    "\n"
                    "MR::payload [length]\n"
                    "\n"
                    "MR::payload\n"
                    "    Returns the collected payload obtained as a result of a prior call to MR::collect.\n"
                    "\n"
                    "MR::payload length\n"
                    "    Returns the length of payload of a MR message."
                ),
                source=_SOURCE,
                examples=(
                    "when MR_DATA {\n"
                    '                log local0 "Payload: [MR::payload]"\n'
                    "            }"
                ),
                return_value="When called without an argument, this command returns the collected payload of an MR message.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::payload ( 'length' )?",
                    arg_values={
                        0: (_av("length", "MR::payload length", "MR::payload ( 'length' )?"),)
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
