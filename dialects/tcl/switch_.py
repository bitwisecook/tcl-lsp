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

"""switch -- Pattern-based branching on a subject string."""

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
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl switch(1)"

_SWITCH_VALUE_OPTIONS = frozenset({"-matchvar", "-indexvar"})


_BODY = frozenset({ArgRole.BODY})


def _switch_arg_roles(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """Resolve BODY roles for switch command."""
    # Skip option flags.
    i = 0
    while i < len(args):
        a = args[i]
        if a == "--":
            i += 1
            break
        if not a.startswith("-"):
            break
        if a in _SWITCH_VALUE_OPTIONS:
            i += 2
        else:
            i += 1
    # Skip switch value.
    if i < len(args):
        i += 1
    if i >= len(args):
        return {}
    roles: dict[int, frozenset[ArgRole]] = {}
    # Braced list form: single trailing argument.
    if i == len(args) - 1:
        roles[i] = _BODY
        return roles
    # List form: pattern body pairs.
    while i + 1 < len(args):
        if args[i + 1] != "-":
            roles[i + 1] = _BODY
        i += 2
    return roles


@register
class SwitchCommand(CommandDef):
    name = "switch"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="switch",
            byte_compiled=True,
            not_proc_factory=True,
            is_control_flow=True,
            is_language_keyword=True,
            never_inline_body=True,
            hover=HoverSnippet(
                summary="Pattern-based branching on a subject string.",
                synopsis=(
                    "switch ?options? string pattern body ?pattern body ...?",
                    "switch ?options? string {pattern body ?pattern body ...?}",
                ),
                snippet="Use `-exact`, `-glob`, or `-regexp` to select matching mode.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="switch ?options? string pattern body ?pattern body ...?",
                    options=(
                        OptionSpec(name="-exact", detail="Exact string compare mode."),
                        OptionSpec(name="-glob", detail="Glob pattern mode."),
                        OptionSpec(name="-regexp", detail="Regular expression mode."),
                        # -nocase / -matchvar / -indexvar: Tcl 8.5+.
                        OptionSpec(
                            name="-nocase",
                            detail="Case-insensitive matching (Tcl 8.5+).",
                            dialects=frozenset({"tcl8.5", "tcl8.6", "tcl9.0"}),
                        ),
                        OptionSpec(
                            name="-matchvar",
                            detail="Store match in variable, regexp mode (Tcl 8.5+).",
                            takes_value=True,
                            value_hint="varName",
                            dialects=frozenset({"tcl8.5", "tcl8.6", "tcl9.0"}),
                        ),
                        OptionSpec(
                            name="-indexvar",
                            detail="Store match indices in variable, regexp mode (Tcl 8.5+).",
                            takes_value=True,
                            value_hint="varName",
                            dialects=frozenset({"tcl8.5", "tcl8.6", "tcl9.0"}),
                        ),
                        OptionSpec(name="--", detail="End of options."),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            arg_role_resolver=_switch_arg_roles,
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
            has_switch_body=True,
        )
