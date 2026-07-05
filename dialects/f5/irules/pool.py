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

"""pool -- Select a load-balancing pool for the current flow."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/pool.html"


@register
class PoolCommand(CommandDef):
    name = "pool"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="pool",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Select a load-balancing pool for the current flow.",
                synopsis=("pool pool_name ?member_addr member_port?",),
                snippet="Can direct traffic to a pool, optionally pinning to a specific member.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT, synopsis="pool pool_name ?member_addr member_port?"
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 3)),
            event_requires=EventRequires(client_side=True),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.POOL_SELECTION,
                    writes=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
