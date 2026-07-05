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

# Scaffolded from try.n -- refine and commit
"""try -- Trap and process errors and exceptions."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page try.n"


_BODY = frozenset({ArgRole.BODY})
_KEYWORD = frozenset({ArgRole.KEYWORD})


def _try_arg_roles(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """Resolve BODY and structural-KEYWORD roles for try/on/trap/finally."""
    roles: dict[int, frozenset[ArgRole]] = {}
    if args:
        roles[0] = _BODY
    i = 1
    while i < len(args):
        kw = args[i]
        if kw == "finally" and i + 1 < len(args):
            roles[i] = _KEYWORD
            roles[i + 1] = _BODY
            i += 2
        elif kw in ("on", "trap") and i + 3 < len(args):
            roles[i] = _KEYWORD
            # A handler body of literal ``-`` is a fallthrough marker
            # (shares the next handler's body, like ``switch``); it is
            # not a script, so it gets no BODY role.
            if args[i + 3] != "-":
                roles[i + 3] = _BODY
            i += 4
        else:
            i += 1
    return roles


@register
class TryCommand(CommandDef):
    name = "try"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="try",
            byte_compiled=True,
            not_proc_factory=True,
            is_control_flow=True,
            is_language_keyword=True,
            never_inline_body=True,
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="Trap and process errors and exceptions",
                synopsis=("try body ?handler...? ?finally script?",),
                snippet="This command executes the script body and, depending on what the outcome of that script is (normal exit, error, or some other exceptional result), runs a handler script to deal with the case.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="try body ?handler...? ?finally script?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            arg_role_resolver=_try_arg_roles,
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
