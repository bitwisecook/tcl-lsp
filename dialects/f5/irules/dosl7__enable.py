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
"""DOSL7::enable -- Enables blocking and detection of DoS attacks according to the ASM security policy configuration."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DOSL7__enable.html"


@register
class Dosl7EnableCommand(CommandDef):
    name = "DOSL7::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DOSL7::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables blocking and detection of DoS attacks according to the ASM security policy configuration.",
                synopsis=("DOSL7::enable (DOSL7_PROFILE_OBJ)?",),
                snippet=(
                    "Enables blocking and detection of DoS attacks according to the ASM\n"
                    "security policy configuration. When disabled using DOSL7::disable,\n"
                    "transactions will bypass DoS L7 for both detection and prevention."
                ),
                source=_SOURCE,
                examples=("when HTTP_REQUEST {\n    DOSL7::enable\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DOSL7::enable (DOSL7_PROFILE_OBJ)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DOSL7_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
