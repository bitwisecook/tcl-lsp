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

"""disconnect -- Disconnect the process from the controlling terminal."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity

from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect disconnect(1)"


@register
class DisconnectCommand(CommandDef):
    name = "disconnect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="disconnect",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Disconnect the process from the controlling terminal (daemonise).",
                synopsis=("disconnect",),
                snippet=(
                    "Disconnects the forked process from the terminal. "
                    "Typically used after ``fork`` in the child process."
                ),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="disconnect"),),
            validation=ValidationSpec(arity=Arity(0, 0)),
        )
