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

# Scaffolded from tell.n -- refine and commit
"""tell -- Return current access position for an open channel."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.dialects import DIALECTS_EXCEPT_IRULES
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
    WasmRuntimeImport,
)
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page tell.n"


@register
class TellCommand(CommandDef):
    name = "tell"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tell",
            byte_compiled=True,
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Return current access position for an open channel",
                synopsis=("tell channel",),
                snippet="The tell command has been superceded by the chan tell command which supports the same syntax and options.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="tell channel",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 1),
            ),
            arg_roles={0: frozenset({ArgRole.CHANNEL})},
            return_type=TclType.INT,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_tell",
                argc=1,
                params=("i32",),
                results=("i32",),
            ),
        )
