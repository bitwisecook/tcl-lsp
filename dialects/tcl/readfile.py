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

"""readFile -- Read the contents of a text or binary file (Tcl 9.0+, TIP 670)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    ArgumentValueSpec,
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
)
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page library.n"

_MODES = (
    ArgumentValueSpec(value="text", detail="Read using system defaults for text files (default)."),
    ArgumentValueSpec(value="binary", detail="Read as uninterpreted bytes."),
)


@register
class ReadFileCommand(CommandDef):
    name = "readFile"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="readFile",
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="Read the contents of a file and return them.",
                synopsis=("readFile filename ?text|binary?",),
                snippet=(
                    "Reads the file *filename* and returns its contents. The "
                    "optional mode is `text` (default; system text-file "
                    "defaults, includes any trailing newline) or `binary` "
                    "(uninterpreted bytes). The file is closed before the "
                    "procedure returns. Introduced by TIP 670 in Tcl 9.0."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="readFile filename ?text|binary?",
                    arg_values={1: _MODES},
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 2),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
