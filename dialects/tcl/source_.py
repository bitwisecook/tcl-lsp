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

"""source -- Evaluate a file or resource as a Tcl script."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.dialects import DIALECTS_EXCEPT_IRULES
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
    WasmRuntimeImport,
)
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl source(1)"


@register
class SourceCommand(CommandDef):
    name = "source"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="source",
            byte_compiled=True,
            not_proc_factory=True,
            is_language_keyword=True,
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Evaluate a file or resource as a Tcl script.",
                synopsis=("source ?-encoding name? fileName",),
                snippet="",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="source ?-encoding name? fileName",
                    options=(
                        # -encoding: TIP 290, Tcl 8.5+.
                        OptionSpec(
                            name="-encoding",
                            takes_value=True,
                            value_hint="encoding",
                            detail="Specify file encoding (TIP 290, Tcl 8.5+).",
                            dialects=frozenset({"tcl8.5", "tcl8.6", "tcl9.0"}),
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 3),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
            sources_file=True,
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_source",
                argc=1,
                params=("i32",),
                results=("i32",),
            ),
        )
