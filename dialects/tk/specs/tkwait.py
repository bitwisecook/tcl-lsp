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

"""tkwait -- Wait for a variable, visibility, or window change."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity

from ._base import register

_SOURCE = "Tk man page tkwait.n"


@register
class TkwaitCommand(CommandDef):
    name = "tkwait"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tkwait",
            required_package="Tk",
            creates_dynamic_barrier=True,
            hover=HoverSnippet(
                summary="Wait for a variable to be written, a window to be destroyed, or a window to become visible.",
                synopsis=(
                    "tkwait variable name",
                    "tkwait visibility name",
                    "tkwait window name",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="tkwait variable|visibility|window name",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2, 2),
            ),
            arg_roles={1: frozenset({ArgRole.VAR_READ})},
        )
