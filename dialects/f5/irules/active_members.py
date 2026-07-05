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
"""active_members -- Returns the number or list of active members in the specified pool."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/active_members.html"


@register
class ActiveMembersCommand(CommandDef):
    name = "active_members"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="active_members",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the number or list of active members in the specified pool.",
                synopsis=("active_members ('-list')? POOL_OBJ",),
                snippet="Returns the number or list of active members in the specified pool.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    if { [active_members http_pool] >= 2 } {\n"
                    "        pool http_pool\n"
                    "    }\n"
                    "}"
                ),
                return_value="active_members <pool_name> Returns the number of active members in the specified pool.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="active_members ('-list')? POOL_OBJ",
                    options=(
                        OptionSpec(
                            name="-list",
                            detail="Return as list instead of count.",
                            takes_value=False,
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"DNS"}),
                also_in=frozenset({"LB_FAILED", "LB_SELECTED"}),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.POOL_SELECTION,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
