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

"""next -- TclOO command to invoke the next method in the MRO."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity

from ._base import register

_SOURCE = "Tcl man page next.n"


@register
class OoNextCommand(CommandDef):
    name = "next"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="next",
            is_language_keyword=True,
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="invoke the next implementation of a method",
                synopsis=("next ?arg ...?",),
                snippet=(
                    "The next command is used within the body of a method to call the "
                    "next implementation of that method in the method resolution order (MRO). "
                    "Arguments are passed to the next implementation."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="next ?arg ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0),
            ),
        )


@register
class OoNextToCommand(CommandDef):
    name = "nextto"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="nextto",
            is_language_keyword=True,
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="invoke a specific superclass implementation of a method",
                synopsis=("nextto class ?arg ...?",),
                snippet=(
                    "The nextto command is like next but invokes a specific class's "
                    "implementation of the current method rather than the next in the MRO."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="nextto class ?arg ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
        )
