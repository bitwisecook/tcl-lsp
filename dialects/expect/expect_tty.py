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

"""expect_tty -- Expect input from the controlling tty."""

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

_SOURCE = "Expect expect_tty(1)"


@register
class ExpectTtyCommand(CommandDef):
    name = "expect_tty"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="expect_tty",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Expect input from the controlling terminal (tty).",
                synopsis=("expect_tty ?-opts? pattern body ?pattern body ...?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="expect_tty ?-opts? pattern body ?pattern body ...?",
                    options=(
                        OptionSpec(name="-re", detail="Match as regular expression."),
                        OptionSpec(name="-ex", detail="Match as exact string."),
                        OptionSpec(name="-gl", detail="Match as glob (default)."),
                        OptionSpec(name="-nocase", detail="Case-insensitive matching."),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
            arg_role_resolver=_expect_arg_roles,
        )
