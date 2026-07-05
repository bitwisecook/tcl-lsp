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
"""LB::persist -- Forces the system to make a persistence decision."""

# Introduced: BIG-IP v9+ (core load-balancing iRules command) (approximate, from F5 documentation)

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
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__persist.html"


_av = make_av(_SOURCE)


@register
class LbPersistCommand(CommandDef):
    name = "LB::persist"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::persist",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Forces the system to make a persistence decision.",
                synopsis=(
                    "LB::persist",
                    "LB::persist key",
                    "LB::persist cookie",
                ),
                snippet=(
                    "This command forces the system to make a persistence decision, and returns a string that can be evaluated to activate that selection, or with the use of the parameter, returns a persistence key that may be used in conjunction with the persist command to manipulate the persistence table.\n"
                    "\n"
                    "This enables an iRule to evaluate the pending load balancing/persistence decision early, and use that information to manage the connection."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::persist ?key | cookie?",
                    arg_values={
                        0: (
                            _av("key", "Get persistence key.", "LB::persist key"),
                            _av("cookie", "Get persistence cookie.", "LB::persist cookie"),
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
                    target=SideEffectTarget.PERSISTENCE_TABLE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
            subcommands={
                "key": SubCommand(
                    name="key",
                    arity=Arity(0, 0),
                    detail="Get persistence key.",
                    synopsis="LB::persist key",
                    pure=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.PERSISTENCE_TABLE,
                            reads=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
                "cookie": SubCommand(
                    name="cookie",
                    arity=Arity(0, 0),
                    detail="Get persistence cookie.",
                    synopsis="LB::persist cookie",
                    pure=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.PERSISTENCE_TABLE,
                            reads=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
            },
        )
