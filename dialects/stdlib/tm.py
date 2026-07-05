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

"""tm -- Tcl module system commands (auto-loaded, no package require needed)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, HoverSnippet, SubCommand, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import register

_SOURCE = "Tcl stdlib tm module system"


@register
class TclTmPath(CommandDef):
    name = "tcl::tm::path"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcl::tm::path",
            hover=HoverSnippet(
                summary="Manage the list of paths searched for Tcl modules.",
                synopsis=(
                    "tcl::tm::path add ?path ...?",
                    "tcl::tm::path remove ?path ...?",
                    "tcl::tm::path list",
                ),
                snippet=(
                    "The ``add`` subcommand prepends paths to the module search "
                    "list, ``remove`` deletes them, and ``list`` returns the "
                    "current list."
                ),
                source=_SOURCE,
            ),
            subcommands={
                "add": SubCommand(
                    name="add",
                    arity=Arity(0),
                ),
                "list": SubCommand(
                    name="list",
                    arity=Arity(0, 0),
                ),
                "remove": SubCommand(
                    name="remove",
                    arity=Arity(0),
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class TclTmRoots(CommandDef):
    name = "tcl::tm::roots"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcl::tm::roots",
            hover=HoverSnippet(
                summary="Set the root paths for Tcl module discovery.",
                synopsis=("tcl::tm::roots pathList",),
                snippet=(
                    "Given a list of root paths, computes version-specific "
                    "sub-directories and adds them to the module search list."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )
