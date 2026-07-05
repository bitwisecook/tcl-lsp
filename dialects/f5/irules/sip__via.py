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
"""SIP::via -- Gets SIP via header information."""

# Introduced: BIG-IP v10+ (core SIP iRules command) (approximate, from F5 documentation)

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

_SOURCE = "https://clouddocs.f5.com/api/irules/SIP__via.html"


_av = make_av(_SOURCE)

_READ_EFFECT = (
    SideEffect(target=SideEffectTarget.NETWORK_IO, reads=True, connection_side=ConnectionSide.BOTH),
)

_VIA_FIELDS = {
    "proto": "Get Via protocol.",
    "sent_by": "Get Via sent-by field.",
    "received": "Get Via received parameter.",
    "branch": "Get Via branch parameter.",
    "maddr": "Get Via maddr parameter.",
    "ttl": "Get Via ttl parameter.",
}


@register
class SipViaCommand(CommandDef):
    name = "SIP::via"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIP::via",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets SIP via header information.",
                synopsis=(
                    "SIP::via ?field? ?INDEX?",
                    "SIP::via (proto | sent_by | received | branch | maddr | ttl) ?INDEX?",
                ),
                snippet="This set of commands allows you to get information in the SIP via header.",
                source=_SOURCE,
                examples=(
                    "when SIP_RESPONSE {\n"
                    "  log local0. [SIP::via 0]\n"
                    "  SIP::header remove Via 0\n"
                    '  SIP::response rewrite 123 "no xxx"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIP::via ?field? ?INDEX?",
                    arg_values={
                        0: tuple(
                            _av(field, detail, f"SIP::via {field} ?index?")
                            for field, detail in _VIA_FIELDS.items()
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"SIP"})),
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
                    arity=Arity(0, 1),
                    detail=detail,
                    synopsis=f"SIP::via {field} ?index?",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                )
                for field, detail in _VIA_FIELDS.items()
            },
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
