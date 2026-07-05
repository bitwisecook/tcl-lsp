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

"""expect -- Wait for output matching a pattern."""

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
from compiler.registry.signatures import ArgRole, Arity

from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect expect(1)"


_BODY = frozenset({ArgRole.BODY})


def _expect_arg_roles(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """Resolve BODY arg roles for expect pattern/body pairs.

    The expect command uses: expect ?opts? pat1 body1 ?pat2 body2 ...?
    Options: -re, -ex, -gl, -nocase, -timeout N, -i spawn_id, -indices,
    -notransfer, -nobrace.  Patterns and bodies alternate after options.
    The special patterns ``timeout``, ``eof``, ``default``, ``full_buffer``
    and ``null`` are followed by a body.
    """
    roles: dict[int, frozenset[ArgRole]] = {}
    i = 0
    while i < len(args):
        arg = args[i]
        if arg in ("-re", "-ex", "-gl", "-nocase", "-indices", "-notransfer", "-nobrace"):
            i += 1
            continue
        if arg in ("-timeout", "-i"):
            i += 2  # option + value
            continue
        if arg == "--":
            i += 1
            continue
        # At this point, arg is a pattern; the next is the body.
        if i + 1 < len(args):
            roles[i + 1] = _BODY
            i += 2
        else:
            i += 1
    return roles


@register
class ExpectCommand(CommandDef):
    name = "expect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="expect",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Wait for output matching a pattern from a spawned process.",
                synopsis=(
                    "expect ?-opts? pattern body ?pattern body ...?",
                    "expect -re {regexp} { actions }",
                    "expect timeout { timeout_actions }",
                    "expect eof { eof_actions }",
                ),
                snippet=(
                    "Waits until one of the patterns matches the output of the "
                    "current spawned process, then executes the corresponding body. "
                    "Special patterns: ``timeout``, ``eof``, ``default``, "
                    "``full_buffer``, ``null``."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="expect ?-opts? pattern body ?pattern body ...?",
                    options=(
                        OptionSpec(name="-re", detail="Match pattern as a Tcl regular expression."),
                        OptionSpec(name="-ex", detail="Match pattern as an exact string."),
                        OptionSpec(name="-gl", detail="Match pattern as a glob (default)."),
                        OptionSpec(name="-nocase", detail="Case-insensitive matching."),
                        OptionSpec(
                            name="-timeout",
                            takes_value=True,
                            value_hint="seconds",
                            detail="Override the timeout for this expect.",
                        ),
                        OptionSpec(
                            name="-i",
                            takes_value=True,
                            value_hint="spawn_id",
                            detail="Specify the spawn id to expect from.",
                        ),
                        OptionSpec(name="-indices", detail="Store match indices in expect_out."),
                        OptionSpec(name="-notransfer", detail="Do not consume matched output."),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
            arg_role_resolver=_expect_arg_roles,
        )
