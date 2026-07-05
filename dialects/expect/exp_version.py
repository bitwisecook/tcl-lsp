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

"""exp_version -- Query or require an Expect version."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity

from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect exp_version(1)"


@register
class ExpVersionCommand(CommandDef):
    name = "exp_version"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="exp_version",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Query or require a minimum Expect version.",
                synopsis=("exp_version ?version?",),
                snippet=(
                    "Without arguments, returns the current Expect version. "
                    "With a version argument, raises an error if the running "
                    "Expect is older than the specified version."
                ),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="exp_version ?version?"),),
            validation=ValidationSpec(arity=Arity(0, 1)),
        )
