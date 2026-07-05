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

"""my -- TclOO method-context command for the current object."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity

from ._base import register

_SOURCE = "Tcl man page my.n"

_av = make_av(_SOURCE)


@register
class OoMyCommand(CommandDef):
    name = "my"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="my",
            is_language_keyword=True,
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="invoke a method on the current object",
                synopsis=("my method ?arg ...?",),
                snippet=(
                    "The my command is used within the body of a method, constructor, or "
                    "destructor to invoke a method on the current object.  It is equivalent "
                    "to [self] method ?arg ...? but avoids the overhead of determining the "
                    "object name."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="my method ?arg ...?",
                    arg_values={
                        0: (
                            _av(
                                "variable",
                                "Link a local variable to an object variable.",
                                "my variable varName ?varName ...?",
                            ),
                            _av(
                                "varname",
                                "Return the fully qualified name of an object variable.",
                                "my varname varName",
                            ),
                            _av("object", "Return the name of the current object.", "my object"),
                            _av(
                                "class",
                                "Return the name of the class of the current object.",
                                "my class",
                            ),
                            _av(
                                "namespace",
                                "Return the namespace of the current object.",
                                "my namespace",
                            ),
                            _av("destroy", "Destroy the current object.", "my destroy"),
                        ),
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
        )
