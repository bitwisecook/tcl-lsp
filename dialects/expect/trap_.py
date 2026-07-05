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

"""trap -- Trap signals."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity

from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect trap(1)"


@register
class TrapCommand(CommandDef):
    name = "trap"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="trap",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Trap signals and execute a command when they occur.",
                synopsis=(
                    "trap ?command? ?signal ...?",
                    "trap SIG_IGN SIGINT",
                    "trap { puts caught } SIGTERM",
                ),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="trap ?command? ?signal ...?"),),
            validation=ValidationSpec(arity=Arity(0)),
            arg_roles={0: frozenset({ArgRole.BODY})},
        )
