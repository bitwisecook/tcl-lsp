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

# Scaffolded from scan.n -- refine and commit
"""scan -- Parse string using conversion specifiers in the style of sscanf."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register
from .const_fold import fold_scan

_SOURCE = "Tcl man page scan.n"


def _scan_arg_roles(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """D4-F2 closure: scan accepts variable-name args from index 2
    onward to the end of the call (``scan STRING FORMAT ?var var ...?``).
    Rather than hard-coding a finite slot count, return VAR_WRITE for
    every trailing arg dynamically, so calls with 20 / 50 / 100 vars
    don't false-fire W210 on the unmodelled tail."""
    return {i: frozenset({ArgRole.VAR_WRITE}) for i in range(2, len(args))}


@register
class ScanCommand(CommandDef):
    name = "scan"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="scan",
            byte_compiled=True,
            hover=HoverSnippet(
                summary="Parse string using conversion specifiers in the style of sscanf",
                synopsis=("scan string format ?varName varName ...?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="scan string format ?varName varName ...?",
                ),
            ),
            arg_role_resolver=_scan_arg_roles,
            const_fold=fold_scan,
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            return_type=TclType.INT,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
