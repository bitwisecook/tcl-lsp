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
"""GTP::clone -- Returns a cloned copy of the GTP message."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/GTP__clone.html"


@register
class GtpCloneCommand(CommandDef):
    name = "GTP::clone"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GTP::clone",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a cloned copy of the GTP message.",
                synopsis=("GTP::clone (MESSAGE_VAR)?",),
                snippet="Returns a cloned copy of the GTP message.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set payload [UDP::payload]\n"
                    "    set t2 [GTP::parse $payload]\n"
                    "    set t3 [GTP::clone $t2]\n"
                    '    log local0. "GTP type [GTP::header type -message $t3]"\n'
                    '    log local0. "GTP teid [GTP::header teid -message $t3]"\n'
                    "}"
                ),
                return_value="Returns a cloned copy of the GTP message.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GTP::clone (MESSAGE_VAR)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
