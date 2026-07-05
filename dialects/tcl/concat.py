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

# Scaffolded from concat.n -- refine and commit
"""concat -- Join lists together."""

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
from compiler.types import TclType

from ._base import register
from .const_fold import fold_concat
from .shimmer_resolvers import resolve_concat

_SOURCE = "Tcl man page concat.n"


@register
class ConcatCommand(CommandDef):
    name = "concat"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="concat",
            frameless_runtime=True,
            hover=HoverSnippet(
                summary="Join lists together",
                synopsis=("concat ?arg arg ...?",),
                snippet="This command joins each of its arguments together with spaces after trimming leading and trailing white-space from each of them.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="concat ?arg arg ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            pure=True,
            const_fold=fold_concat,
            produces_canonical_list=True,
            return_type=TclType.LIST,
            arg_type_resolver=resolve_concat,
            side_effect_hints=(),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_concat",
                argc=2,
                params=("i32", "i32"),
                results=("i32",),
            ),
        )
