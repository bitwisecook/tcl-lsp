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
"""DIAMETER::header -- Gets or sets the DIAMETER header fields."""

# Introduced: BIG-IP v11+ (Diameter module) (approximate, from F5 documentation)

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__header.html"


_av = make_av(_SOURCE)

_READ_EFFECT = (
    SideEffect(
        target=SideEffectTarget.NETWORK_IO,
        reads=True,
        connection_side=ConnectionSide.BOTH,
    ),
)
_WRITE_EFFECT = (
    SideEffect(
        target=SideEffectTarget.NETWORK_IO,
        reads=True,
        writes=True,
        connection_side=ConnectionSide.BOTH,
    ),
)

# Fields that can be get/set with 0 or 1 args
_HEADER_FIELDS = {
    "version": "Get/set the DIAMETER protocol version.",
    "length": "Get the message length (read-only).",
    "rflag": "Get/set the request flag (R-bit).",
    "pflag": "Get/set the proxiable flag (P-bit).",
    "eflag": "Get/set the error flag (E-bit).",
    "tflag": "Get/set the retransmit flag (T-bit).",
    "command_code": "Get/set the command code.",
    "application_id": "Get/set the application ID.",
    "hop_by_hop_id": "Get/set the hop-by-hop identifier.",
    "end_to_end_id": "Get/set the end-to-end identifier.",
}


@register
class DiameterHeaderCommand(CommandDef):
    name = "DIAMETER::header"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::header",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the DIAMETER header fields.",
                synopsis=(
                    "DIAMETER::header <field> ?value?",
                    "DIAMETER::header command_code ?value?",
                    "DIAMETER::header application_id ?value?",
                ),
                snippet="This iRule command is used to get and set header fields in the current DIAMETER message.",
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n"
                    "    if { [DIAMETER::header tflag] } {\n"
                    '        log local0. "Received a potentially retransmitted Diameter message"\n'
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::header <field> ?value?",
                    arg_values={
                        0: tuple(
                            _av(field, detail, f"DIAMETER::header {field} ?value?")
                            for field, detail in _HEADER_FIELDS.items()
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DIAMETER", "MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
            subcommands={
                field: SubCommand(
                    name=field,
                    arity=Arity(0, 0) if field == "length" else Arity(0, 1),
                    detail=detail,
                    synopsis=f"DIAMETER::header {field} ?value?",
                    pure=True,
                    mutator=field != "length",
                    side_effect_hints=_READ_EFFECT if field == "length" else _WRITE_EFFECT,
                )
                for field, detail in _HEADER_FIELDS.items()
            },
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
