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
"""SSL::sni -- Returns Server Name Indication information."""

# Introduced: BIG-IP v12+ (Server Name Indication) (approximate, from F5 documentation)

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

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__sni.html"


_av = make_av(_SOURCE)

_READ_EFFECT = (
    SideEffect(target=SideEffectTarget.SSL_STATE, reads=True, connection_side=ConnectionSide.BOTH),
)


@register
class SslSniCommand(CommandDef):
    name = "SSL::sni"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::sni",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns Server Name Indication information.",
                synopsis=("SSL::sni (name | required)",),
                snippet="Returns a Server Name Indication name, and require SNI support.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    log local0.info "SNI name: [SSL::sni name]"\n'
                    '    log local0.info "SNI required: [SSL::sni required]"\n'
                    "}"
                ),
                return_value="SSL::sni name Returns the current Server Name Indication as specified in the SSL profile. SSL::sni required Returns the require SNI support as specified in the SSL profile.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::sni <name | required>",
                    arg_values={
                        0: (
                            _av("name", "Get SNI name.", "SSL::sni name"),
                            _av("required", "Get SNI required setting.", "SSL::sni required"),
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
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
            subcommands={
                "name": SubCommand(
                    name="name",
                    arity=Arity(0, 0),
                    detail="Get SNI name.",
                    synopsis="SSL::sni name",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "required": SubCommand(
                    name="required",
                    arity=Arity(0, 0),
                    detail="Get SNI required setting.",
                    synopsis="SSL::sni required",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
            },
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(
            source={Arity(0, 0): TaintColour.TAINTED | TaintColour.FQDN},
        )
