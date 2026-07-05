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

"""self -- TclOO command to query the current object identity."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page self.n"

_av = make_av(_SOURCE)


@register
class OoSelfCommand(CommandDef):
    name = "self"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="self",
            is_language_keyword=True,
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="query the identity of the current object",
                synopsis=("self ?subcommand?",),
                snippet=(
                    "The self command is used within the body of a method to query the "
                    "identity or other properties of the current object."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="self ?subcommand?",
                    arg_values={
                        0: (
                            _av("object", "Return the name of the current object.", "self object"),
                            _av("class", "Return the class of the current object.", "self class"),
                            _av(
                                "namespace",
                                "Return the namespace of the current object.",
                                "self namespace",
                            ),
                            _av("method", "Return the name of the current method.", "self method"),
                            _av("caller", "Return info about the calling method.", "self caller"),
                            _av(
                                "target",
                                "Return the name of the target of a forward method.",
                                "self target",
                            ),
                            _av("call", "Return the current call chain.", "self call"),
                            _av("filter", "Return the current filter.", "self filter"),
                        ),
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0, 1),
            ),
            return_type=TclType.STRING,
        )
