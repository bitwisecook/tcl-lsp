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

"""classvariable -- TclOO command to link class-level variables."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity

from ._base import register

_SOURCE = "Tcl man page define.n"


@register
class OoClassvariableCommand(CommandDef):
    name = "classvariable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="classvariable",
            is_language_keyword=True,
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="link local variables to class-shared variables",
                synopsis=("classvariable variableName ?variableName ...?",),
                snippet=(
                    "The classvariable command arranges for local variables with the given "
                    "names to refer to variables shared among all instances of the class. "
                    "Used inside method bodies or class definitions."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="classvariable variableName ?variableName ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
        )
