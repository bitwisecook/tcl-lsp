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

# Scaffolded from format.n -- refine and commit
"""format -- Format a string in the style of sprintf."""

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
from .const_fold import fold_format

_SOURCE = "Tcl man page format.n"


@register
class FormatCommand(CommandDef):
    name = "format"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="format",
            byte_compiled=True,
            hover=HoverSnippet(
                summary="Format a string in the style of sprintf",
                synopsis=("format formatString ?arg arg ...?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="format formatString ?arg arg ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            pure=True,
            const_fold=fold_format,
            cse_candidate=True,
            return_type=TclType.STRING,
            side_effect_hints=(),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_format",
                argc=4,
                params=("i32", "i32", "i32", "i32"),
                results=("i32",),
            ),
        )
