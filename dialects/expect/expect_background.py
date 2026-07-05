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

"""expect_background -- Non-blocking expect."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from compiler.registry.signatures import Arity

from ._base import _EXPECT_ONLY, register
from .expect_ import _expect_arg_roles

_SOURCE = "Expect expect_background(1)"


@register
class ExpectBackgroundCommand(CommandDef):
    name = "expect_background"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="expect_background",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Non-blocking expect: run pattern matching in the background.",
                synopsis=("expect_background ?-opts? pattern body ?pattern body ...?",),
                snippet=(
                    "Like ``expect`` but does not block. Whenever new data arrives "
                    "the patterns are tested and the matching body is executed."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="expect_background ?-opts? pattern body ?pattern body ...?",
                    options=(
                        OptionSpec(name="-re", detail="Match as regular expression."),
                        OptionSpec(name="-ex", detail="Match as exact string."),
                        OptionSpec(name="-gl", detail="Match as glob (default)."),
                        OptionSpec(name="-nocase", detail="Case-insensitive matching."),
                        OptionSpec(
                            name="-i",
                            takes_value=True,
                            value_hint="spawn_id",
                            detail="Specify the spawn id.",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
            arg_role_resolver=_expect_arg_roles,
        )
