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
"""ASM::enable -- Enables plugin processing on the connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__enable.html"


@register
class AsmEnableCommand(CommandDef):
    name = "ASM::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables plugin processing on the connection.",
                synopsis=("ASM::enable ASM_POLICY",),
                snippet=(
                    "Enables the ASM plugin processing for the current TCP connection.\n"
                    "ASM will remain enabled on the current TCP connection until it is closed or\n"
                    "ASM::disable is called."
                ),
                source=_SOURCE,
                examples=(
                    "#Enabling an asm policy called asmb in the Common partition in v11.4.x and Later\n"
                    "when HTTP_REQUEST {\n"
                    '    ASM::enable "/Common/asmb"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::enable ASM_POLICY",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"FASTHTTP"})),
            xc_translatable=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
