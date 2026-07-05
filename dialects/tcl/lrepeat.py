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

"""lrepeat -- Build a list by repeating elements."""

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
from .const_fold import fold_lrepeat

_SOURCE = "Tcl man page lrepeat.n"


@register
class LrepeatCommand(CommandDef):
    name = "lrepeat"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lrepeat",
            frameless_runtime=True,
            hover=HoverSnippet(
                summary="Build a list by repeating elements",
                synopsis=("lrepeat count ?element ...?",),
                snippet="The lrepeat command creates a list of size count * number of elements by repeating count times the sequence of elements element ....",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lrepeat count ?element ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            pure=True,
            const_fold=fold_lrepeat,
            return_type=TclType.LIST,
            inferred_storage_type=StorageType.LIST,
            arg_types={0: ArgTypeHint(expected=TclType.INT, shimmers=True)},
            side_effect_hints=(),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_list_repeat",
                argc=2,
                params=("i32", "i32"),
                results=("i32",),
            ),
        )
