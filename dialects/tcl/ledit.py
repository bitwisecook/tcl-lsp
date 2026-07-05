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

# Scaffolded from lreplace.n -- refine and commit
"""ledit -- In-place list edit; like lreplace but mutates a variable."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
)
from compiler.registry.signatures import ArgRole, Arity
from compiler.registry.type_hints import ArgTypeHint
from compiler.side_effects import StorageType
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl 9 man page ledit.n"


@register
class LeditCommand(CommandDef):
    name = "ledit"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ledit",
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="Replace elements in a list variable in place",
                synopsis=("ledit listVar first last ?element element ...?",),
                snippet=(
                    "ledit takes a variable named listVar containing a list, "
                    "and replaces the range from first to last with the "
                    "element arguments.  The variable is updated and the new "
                    "value is also returned.  Introduced in Tcl 9.0."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ledit listVar first last ?element element ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(3),
            ),
            pure=False,
            return_type=TclType.LIST,
            inferred_storage_type=StorageType.LIST,
            assigns_variable_at=0,
            arg_types={
                1: ArgTypeHint(expected=TclType.INT, shimmers=True),
                2: ArgTypeHint(expected=TclType.INT, shimmers=True),
            },
            arg_roles={0: frozenset({ArgRole.VAR_WRITE})},
            side_effect_hints=(),
        )
