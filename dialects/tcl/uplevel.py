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

# Scaffolded from uplevel.n -- refine and commit
"""uplevel -- Execute a script in a different stack frame."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page uplevel.n"


@register
class UplevelCommand(CommandDef):
    name = "uplevel"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="uplevel",
            byte_compiled=True,
            not_proc_factory=True,
            is_language_keyword=True,
            creates_dynamic_barrier=True,
            unsafe=True,
            hover=HoverSnippet(
                summary="Execute a script in a different stack frame",
                synopsis=("uplevel ?level? arg ?arg ...?",),
                snippet="All of the arg arguments are concatenated as if they had been passed to concat; the result is then evaluated in the variable context indicated by level.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="uplevel ?level? arg ?arg ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            taint_sink=True,
            # NOTE (D5-T100/T105): LIST_CANONICAL is NO LONGER a sound
            # suppression for uplevel -- it only proves list-quoting,
            # not that the synthesised command word is trusted.  T100
            # suppression now requires a literal ``[list <known-cmd>
            # ...]`` cmd-sub with the tainted var at index >= 1; see
            # compiler/taint/_sinks.py.
            xc_translatable=False,
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(target=SideEffectTarget.UNKNOWN, connection_side=ConnectionSide.NONE),
            ),
            evaluates_code=True,
        )
