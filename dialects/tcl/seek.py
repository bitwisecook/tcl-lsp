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

"""seek -- Set access position for a channel."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.dialects import DIALECTS_EXCEPT_IRULES
from compiler.registry.models import (
    ArgumentValueSpec,
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

_SOURCE = "Tcl man page seek.n"


def _origin(value: str, detail: str) -> ArgumentValueSpec:
    return ArgumentValueSpec(
        value=value,
        detail=detail,
        hover=HoverSnippet(
            summary=detail,
            synopsis=(f"seek channelId offset {value}",),
            source=_SOURCE,
        ),
    )


_ORIGINS = (
    _origin("start", "Offset is relative to the beginning of the channel."),
    _origin("current", "Offset is relative to the current access position."),
    _origin("end", "Offset is relative to the end of the channel."),
)


@register
class SeekCommand(CommandDef):
    name = "seek"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="seek",
            byte_compiled=True,
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Set the access position for a channel.",
                synopsis=("seek channelId offset ?origin?",),
                snippet="Default origin is `start`. Returns empty string.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="seek channelId offset ?origin?",
                    arg_values={2: _ORIGINS},
                ),
            ),
            validation=ValidationSpec(arity=Arity(2, 3)),
            arg_roles={0: frozenset({ArgRole.CHANNEL})},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_seek",
                argc=3,
                params=("i32", "i32", "i32"),
                results=("i32",),
            ),
        )
