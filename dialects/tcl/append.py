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

# Scaffolded from append.n -- refine and commit
"""append -- Append to variable."""

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
from compiler.registry.signatures import ArgRole, Arity
from compiler.registry.type_hints import ArgTypeHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page append.n"


@register
class AppendCommand(CommandDef):
    name = "append"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="append",
            byte_compiled=True,
            frameless_runtime=True,
            hover=HoverSnippet(
                summary="Append to variable",
                synopsis=("append varName ?value value value ...?",),
                snippet="Append all of the value arguments to the current value of variable varName.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="append varName ?value value value ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_append",
                argc=2,
                nontrapping=True,
                params=("i32", "i32"),
                results=("i32",),
            ),
            assigns_variable_at=0,
            reads_variable_before_write=True,
            safe_on_uninit=frozenset(),
            arg_roles={0: frozenset({ArgRole.VAR_WRITE})},
            return_type=TclType.STRING,
            arg_types={0: ArgTypeHint(expected=TclType.STRING, shimmers=True)},
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
            has_string_list_confusion_risk=True,
        )
