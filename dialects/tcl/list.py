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

# Scaffolded from list.n -- refine and commit
"""list -- Create a list."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.types import TclType

from ._base import register
from .const_fold import fold_list

_SOURCE = "Tcl man page list.n"


@register
class ListCommand(CommandDef):
    name = "list"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="list",
            byte_compiled=True,
            frameless_runtime=True,
            not_proc_factory=True,
            hover=HoverSnippet(
                summary="Create a list",
                synopsis=("list ?arg arg ...?",),
                snippet="This command returns a list comprised of all the args, or an empty string if no args are specified.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="list ?arg arg ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            pure=True,
            const_fold=fold_list,
            produces_canonical_list=True,
            return_type=TclType.LIST,
            side_effect_hints=(),
        )
