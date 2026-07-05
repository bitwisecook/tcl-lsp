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

"""puts -- Write text to a channel."""

from __future__ import annotations

from compiler.registry._base import CommandDef
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

_SOURCE = "Tcl puts(1)"


@register
class PutsCommand(CommandDef):
    name = "puts"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="puts",
            byte_compiled=True,
            frameless_runtime=True,
            hover=HoverSnippet(
                summary="Write text to a channel (stdout by default).",
                synopsis=("puts ?-nonewline? ?channelId? string",),
                snippet="Use `-nonewline` to suppress the trailing newline.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="puts ?-nonewline? ?channelId? string",
                    options=(
                        OptionSpec(
                            name="-nonewline",
                            detail="Do not append trailing newline.",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                # ``puts ?-nonewline? ?channelId? string`` accepts
                # 1, 2, or 3 raw args, but the static arity is
                # ``Arity(1, 2)`` *positional* — the analyser strips
                # the leading ``-nonewline`` flag (declared on
                # ``CommandSig.options``) before counting, so
                # ``puts -nonewline $chan msg`` (3 raw / 2 positional)
                # passes while ``puts a b c`` (3 positional) is
                # rejected as too many.  The Zig handler accepts the
                # 3-arg shape internally regardless of this static
                # bound; the runtime doesn't enforce ``arity_max``.
                arity=Arity(1, 2),
            ),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_puts",
                argc=1,
                nontrapping=True,
                params=("i32",),
                results=("i32",),
            ),
            taint_output_sink="T101",
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
