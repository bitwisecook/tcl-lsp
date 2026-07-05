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

# Scaffolded from llength.n -- refine and commit
"""llength -- Count the number of elements in a list."""

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
from .const_fold import fold_llength

_SOURCE = "Tcl man page llength.n"


@register
class LlengthCommand(CommandDef):
    name = "llength"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="llength",
            byte_compiled=True,
            frameless_runtime=True,
            hover=HoverSnippet(
                summary="Count the number of elements in a list",
                synopsis=("llength list",),
                snippet="Treats list as a list and returns a decimal string giving the number of elements in it.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="llength list",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 1),
            ),
            pure=True,
            const_fold=fold_llength,
            cse_candidate=True,
            return_type=TclType.INT,
            arg_types={0: ArgTypeHint(expected=TclType.LIST, shimmers=True)},
            side_effect_hints=(),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_list_length",
                argc=1,
                params=("i32",),
                results=("i32",),
            ),
        )
