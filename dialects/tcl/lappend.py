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

"""lappend -- Append list elements onto a variable."""

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
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageType
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page lappend.n"


@register
class LappendCommand(CommandDef):
    name = "lappend"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lappend",
            byte_compiled=True,
            frameless_runtime=True,
            not_proc_factory=True,
            hover=HoverSnippet(
                summary="Append list elements onto a variable",
                synopsis=("lappend varName ?value value value ...?",),
                snippet="This command treats the variable given by varName as a list and appends each of the value arguments to that list as a separate element, with spaces between elements.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lappend varName ?value value value ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            assigns_variable_at=0,
            reads_variable_before_write=True,
            safe_on_uninit=frozenset(),
            arg_roles={0: frozenset({ArgRole.VAR_WRITE})},
            return_type=TclType.LIST,
            inferred_storage_type=StorageType.LIST,
            arg_types={0: ArgTypeHint(expected=TclType.LIST, shimmers=True)},
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_lappend",
                argc=2,
                params=("i32", "i32"),
                results=("i32",),
            ),
        )
