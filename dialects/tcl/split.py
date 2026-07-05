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

# Scaffolded from split.n -- refine and commit
"""split -- Split a string into a proper Tcl list."""

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
from compiler.types import TclType

from ._base import register
from .const_fold import fold_split

_SOURCE = "Tcl man page split.n"


@register
class SplitCommand(CommandDef):
    name = "split"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="split",
            frameless_runtime=True,
            hover=HoverSnippet(
                summary="Split a string into a proper Tcl list",
                synopsis=("split string ?splitChars?",),
                snippet="Returns a list created by splitting string at each character that is in the splitChars argument.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="split string ?splitChars?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 2),
            ),
            pure=True,
            const_fold=fold_split,
            cse_candidate=True,
            return_type=TclType.LIST,
            arg_types={0: ArgTypeHint(expected=TclType.STRING, shimmers=True)},
            side_effect_hints=(),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_split",
                argc=2,
                params=("i32", "i32"),
                results=("i32",),
            ),
        )
