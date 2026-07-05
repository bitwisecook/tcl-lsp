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
"""GTP::header -- Allows for the parsing of GTP header information."""

# Introduced: BIG-IP v11+ (GTP module) (approximate, from F5 documentation)

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/GTP__header.html"


_av = make_av(_SOURCE)

_READ_EFFECT = (
    SideEffect(target=SideEffectTarget.NETWORK_IO, reads=True, connection_side=ConnectionSide.BOTH),
)
_WRITE_EFFECT = (
    SideEffect(
        target=SideEffectTarget.NETWORK_IO,
        reads=True,
        writes=True,
        connection_side=ConnectionSide.BOTH,
    ),
)

# Fields that support get/set/remove
_MUTABLE_FIELDS = {"teid", "npdu", "sequence"}
# Read-only fields
_READONLY_FIELDS = {"version", "type"}


@register
class GtpHeaderCommand(CommandDef):
    name = "GTP::header"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GTP::header",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Allows for the parsing of GTP header information.",
                synopsis=(
                    "GTP::header ('version' | 'type') ('-message' MESSAGE)?",
                    "GTP::header ('teid' | 'npdu' | 'sequence') ('-message' MESSAGE)?",
                    "GTP::header ('teid' | 'npdu' | 'sequence') 'set' ('-message' MESSAGE)? VALUE",
                    "GTP::header ('teid' | 'npdu' | 'sequence') 'remove' ('-message' MESSAGE)?",
                ),
                snippet=(
                    "Allows for the parsing of GTP header information. UINT -- Unsigned\n"
                    "integer value of n bits. For n > 8, appropriate network to host byte\n"
                    "order conversion happens transparently."
                ),
                source=_SOURCE,
                examples=(
                    "when GTP_SIGNALLING_INGRESS {\n"
                    '    log local0. "GTP version [GTP::header version]"\n'
                    '    log local0. "GTP type [GTP::header type]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GTP::header <field> ?set|remove? ?-message msg? ?value?",
                    options=(
                        OptionSpec(
                            name="-message",
                            detail="Operate on specific message.",
                            takes_value=True,
                            value_hint="MESSAGE",
                        ),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "version", "Get GTP version.", "GTP::header version ?-message msg?"
                            ),
                            _av("type", "Get GTP message type.", "GTP::header type ?-message msg?"),
                            _av(
                                "teid",
                                "Get/set/remove TEID.",
                                "GTP::header teid ?set|remove? ?-message msg? ?value?",
                            ),
                            _av(
                                "npdu",
                                "Get/set/remove N-PDU number.",
                                "GTP::header npdu ?set|remove? ?-message msg? ?value?",
                            ),
                            _av(
                                "sequence",
                                "Get/set/remove sequence number.",
                                "GTP::header sequence ?set|remove? ?-message msg? ?value?",
                            ),
                            _av(
                                "extension",
                                "Access extension headers.",
                                "GTP::header extension ?args?",
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
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
            subcommands={
                **{
                    field: SubCommand(
                        name=field,
                        arity=Arity(0, 0),
                        detail=f"Get GTP {field}.",
                        synopsis=f"GTP::header {field} ?-message msg?",
                        pure=True,
                        options=(
                            OptionSpec(
                                name="-message",
                                detail="Operate on specific message.",
                                takes_value=True,
                                value_hint="MESSAGE",
                            ),
                        ),
                        side_effect_hints=_READ_EFFECT,
                    )
                    for field in _READONLY_FIELDS
                },
                **{
                    field: SubCommand(
                        name=field,
                        arity=Arity(0),
                        detail=f"Get/set/remove GTP {field}.",
                        synopsis=f"GTP::header {field} ?set|remove? ?-message msg? ?value?",
                        pure=True,
                        mutator=True,
                        options=(
                            OptionSpec(
                                name="-message",
                                detail="Operate on specific message.",
                                takes_value=True,
                                value_hint="MESSAGE",
                            ),
                        ),
                        side_effect_hints=_WRITE_EFFECT,
                    )
                    for field in _MUTABLE_FIELDS
                },
                "extension": SubCommand(
                    name="extension",
                    arity=Arity(0),
                    detail="Access GTP extension headers.",
                    synopsis="GTP::header extension ?args?",
                    pure=True,
                    mutator=True,
                    options=(
                        OptionSpec(
                            name="-message",
                            detail="Operate on specific message.",
                            takes_value=True,
                            value_hint="MESSAGE",
                        ),
                    ),
                    side_effect_hints=_WRITE_EFFECT,
                ),
            },
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
