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
"""proc -- Define an iRule proc."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import ArgRole, Arity, BodyKind
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/proc.html"


@register
class ProcCommand(CommandDef):
    name = "proc"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="proc",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Define an iRule proc.",
                synopsis=("proc NAME ARGUMENT_N_DEFAULT PROC_SCRIPT",),
                snippet=(
                    "Define an iRule proc which is called by iRule command call.\n"
                    "\n"
                    "The syntax is same as basic TCL proc command."
                ),
                source=_SOURCE,
                examples=('when CLIENT_DATA {\n    call logme "Coming to CLIENT_DATA"\n}'),
                return_value="Returns the value in the return command, if any, in the proc script.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="proc NAME ARGUMENT_N_DEFAULT PROC_SCRIPT",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(3, 3),
            ),
            event_requires=EventRequires(),
            arg_roles={
                0: frozenset({ArgRole.NAME}),
                1: frozenset({ArgRole.PARAM_LIST}),
                2: frozenset({ArgRole.BODY}),
            },
            body_kind=BodyKind.STRUCTURAL,
            defines_procedure=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.PROC_DEFINITION,
                    writes=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
