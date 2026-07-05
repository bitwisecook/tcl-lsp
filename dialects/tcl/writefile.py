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

"""writeFile -- Write contents to a text or binary file (Tcl 9.0+, TIP 670)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page library.n"


@register
class WriteFileCommand(CommandDef):
    name = "writeFile"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="writeFile",
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="Write contents to a file.",
                synopsis=("writeFile filename ?text|binary? contents",),
                snippet=(
                    "Writes *contents* to the file *filename*. The optional "
                    "mode is `text` (default; system text-file defaults) or "
                    "`binary` (uninterpreted bytes). A trailing newline, if "
                    "needed, must be included in *contents*. The result is "
                    "the empty string; the file is closed before the "
                    "procedure returns. Introduced by TIP 670 in Tcl 9.0."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="writeFile filename ?text|binary? contents",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2, 3),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
