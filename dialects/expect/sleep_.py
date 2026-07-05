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

"""sleep -- Pause execution for a number of seconds."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity

from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect sleep(1)"


@register
class SleepCommand(CommandDef):
    name = "sleep"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="sleep",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Pause execution for the specified number of seconds.",
                synopsis=("sleep seconds",),
                snippet="Accepts integer or decimal values (e.g. ``sleep 0.5``).",
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="sleep seconds"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )
