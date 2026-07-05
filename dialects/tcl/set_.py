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

"""set -- Read or write a variable value."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl set(1)"


def _set_arg_role_resolver(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """Resolve variable role based on the shape of ``set``.

    ``set varName``       -- read: returns the value (VAR_READ)
    ``set varName value`` -- write: assigns and returns (VAR_WRITE)
    """
    if len(args) >= 2:
        return {0: frozenset({ArgRole.VAR_WRITE})}
    return {0: frozenset({ArgRole.VAR_READ})}


@register
class SetCommand(CommandDef):
    name = "set"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="set",
            byte_compiled=True,
            frameless_runtime=True,
            not_proc_factory=True,
            hover=HoverSnippet(
                summary="Read or write a variable value.",
                synopsis=("set varName ?newValue?",),
                snippet="With one argument, returns the value. With two, assigns and returns the new value.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="set varName ?newValue?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 2),
            ),
            assigns_variable_at=0,
            arg_role_resolver=_set_arg_role_resolver,
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
