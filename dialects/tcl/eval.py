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

# Scaffolded from eval.n -- refine and commit
"""eval -- Evaluate a Tcl script."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page eval.n"


@register
class EvalCommand(CommandDef):
    name = "eval"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="eval",
            byte_compiled=True,
            not_proc_factory=True,
            creates_dynamic_barrier=True,
            hover=HoverSnippet(
                summary="Evaluate a Tcl script.",
                synopsis=("eval arg ?arg ...?",),
                snippet=(
                    "Concatenates its arguments and executes the result as a "
                    "Tcl script.\n\n"
                    "**Security**: If any argument contains user-controlled "
                    "data, this enables arbitrary code injection. Prefer "
                    "`{*}$cmdList` (Tcl 8.5+) to expand pre-built command "
                    "lists safely, or use direct invocation."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="eval arg ?arg ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            taint_sink=True,
            # NOTE (D5-T100/T105): LIST_CANONICAL is NO LONGER a sound
            # suppression for eval -- it only proves list-quoting, not
            # that the synthesised command word is trusted.  T100 is
            # instead suppressed only when the literal eval arg is a
            # ``[list <known-cmd> ...]`` cmd-sub with the tainted var at
            # list-index >= 1, in compiler/taint/_sinks.py.
            xc_translatable=False,
            arg_roles={0: frozenset({ArgRole.BODY})},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(target=SideEffectTarget.UNKNOWN, connection_side=ConnectionSide.NONE),
            ),
            evaluates_code=True,
        )
