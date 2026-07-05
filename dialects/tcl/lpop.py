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

"""lpop -- Get and remove an element from a list variable (Tcl 9.0+, TIP 323)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.registry.type_hints import ArgTypeHint
from compiler.side_effects import StorageType
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl 9 man page lpop.n"


@register
class LpopCommand(CommandDef):
    name = "lpop"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lpop",
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="Get and remove an element in a list variable.",
                synopsis=("lpop varName ?index ...?",),
                snippet=(
                    "Removes the element at the given indices from the list "
                    "stored in `varName` (defaulting to `end`) and returns "
                    "it. Nested indices descend into sublists, like `lindex` "
                    "/ `lset`. Introduced in Tcl 9.0."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lpop varName ?index ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            pure=False,
            return_type=TclType.STRING,
            inferred_storage_type=StorageType.LIST,
            assigns_variable_at=0,
            arg_roles={0: frozenset({ArgRole.VAR_WRITE})},
            arg_types={0: ArgTypeHint(expected=TclType.LIST, shimmers=True)},
            side_effect_hints=(),
        )
