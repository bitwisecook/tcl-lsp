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
"""LSN::pool -- Explicitly set the LSN pool used for translation."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LSN__pool.html"


@register
class LsnPoolCommand(CommandDef):
    name = "LSN::pool"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LSN::pool",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Explicitly set the LSN pool used for translation.",
                synopsis=("LSN::pool LSN_POOL",),
                snippet=(
                    "Explicitly set the LSN pool used for translation.\n\nLSN::pool <pool_name>"
                ),
                source=_SOURCE,
                return_value="LSN::pool",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LSN::pool LSN_POOL",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"FASTHTTP", "MR", "RTSP", "SIP"}),
                also_in=frozenset(
                    {"CLIENT_ACCEPTED", "CLIENT_DATA", "LB_FAILED", "LB_SELECTED", "SA_PICKED"}
                ),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.POOL_SELECTION,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
