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

# Scaffolded from apply.n -- refine and commit
"""apply -- Apply an anonymous function."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
    WasmRuntimeImport,
)
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page apply.n"


@register
class ApplyCommand(CommandDef):
    name = "apply"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="apply",
            byte_compiled=True,
            not_proc_factory=True,
            is_language_keyword=True,
            hover=HoverSnippet(
                summary="Apply an anonymous function",
                synopsis=("apply func ?arg1 arg2 ...?",),
                snippet="The command apply applies the function func to the arguments arg1 arg2 ...",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="apply func ?arg1 arg2 ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.PROC_DEFINITION, connection_side=ConnectionSide.NONE
                ),
            ),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_apply",
                argc=2,
                params=("i32", "i32"),
                results=("i32",),
            ),
        )
