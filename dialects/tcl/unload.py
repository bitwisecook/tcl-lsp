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

# Scaffolded from unload.n -- refine and commit
"""unload -- Unload machine code."""

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

_SOURCE = "Tcl man page unload.n"


@register
class UnloadCommand(CommandDef):
    name = "unload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="unload",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Unload machine code",
                synopsis=(
                    "unload ?switches? fileName",
                    "unload ?switches? fileName prefix",
                    "unload ?switches? fileName prefix interp",
                ),
                snippet="This command tries to unload shared libraries previously loaded with load from the application's address space.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="unload ?switches? fileName",
                    options=(
                        OptionSpec(
                            name="-nocomplain",
                            detail="Suppresses all error messages.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="-keeplibrary",
                            detail="This switch will prevent unload from issuing the operating system call that will unload the library from the process.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="--", detail="Marks the end of switches.", takes_value=False
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
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_unload",
                argc=1,
                params=("i32",),
                results=("i32",),
            ),
        )
