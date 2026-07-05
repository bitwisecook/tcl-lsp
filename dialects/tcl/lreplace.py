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

# Scaffolded from lreplace.n -- refine and commit
"""lreplace -- Replace elements in a list with new elements."""

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
from compiler.registry.type_hints import ArgTypeHint
from compiler.side_effects import StorageType
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page lreplace.n"


@register
class LreplaceCommand(CommandDef):
    name = "lreplace"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lreplace",
            byte_compiled=True,
            frameless_runtime=True,
            hover=HoverSnippet(
                summary="Replace elements in a list with new elements",
                synopsis=("lreplace list first last ?element element ...?",),
                snippet="lreplace returns a new list formed by replacing zero or more elements of list with the element arguments.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lreplace list first last ?element element ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(3),
            ),
            pure=True,
            return_type=TclType.LIST,
            inferred_storage_type=StorageType.LIST,
            arg_types={
                0: ArgTypeHint(expected=TclType.LIST, shimmers=True),
                1: ArgTypeHint(expected=TclType.INT, shimmers=True),
                2: ArgTypeHint(expected=TclType.INT, shimmers=True),
            },
            side_effect_hints=(),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_list_replace",
                argc=4,
                params=("i32", "i32", "i32", "i32"),
                results=("i32",),
            ),
        )
