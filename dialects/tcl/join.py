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

# Scaffolded from join.n -- refine and commit
"""join -- Create a string by joining together list elements."""

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
from .const_fold import fold_join

_SOURCE = "Tcl man page join.n"


@register
class JoinCommand(CommandDef):
    name = "join"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="join",
            frameless_runtime=True,
            hover=HoverSnippet(
                summary="Create a string by joining together list elements",
                synopsis=("join list ?joinString?",),
                snippet="The list argument must be a valid Tcl list.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="join list ?joinString?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 2),
            ),
            pure=True,
            const_fold=fold_join,
            cse_candidate=True,
            return_type=TclType.STRING,
            arg_types={0: ArgTypeHint(expected=TclType.LIST, shimmers=True)},
            side_effect_hints=(),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_join",
                argc=2,
                params=("i32", "i32"),
                results=("i32",),
            ),
        )
