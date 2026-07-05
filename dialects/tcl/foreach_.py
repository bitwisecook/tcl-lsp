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

"""foreach -- Iterate over list elements with one or more loop variables."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register
from .shimmer_resolvers import resolve_foreach

_SOURCE = "Tcl foreach(1)"


def _foreach_arg_roles(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """Last argument is the body script."""
    if len(args) >= 3:
        return {len(args) - 1: frozenset({ArgRole.BODY})}
    return {}


@register
class ForeachCommand(CommandDef):
    name = "foreach"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="foreach",
            byte_compiled=True,
            not_proc_factory=True,
            is_control_flow=True,
            is_language_keyword=True,
            never_inline_body=True,
            has_loop_body=True,
            loop_list_header=True,
            hover=HoverSnippet(
                summary="Iterate over list elements with one or more loop variables.",
                synopsis=("foreach varList list ?varList list ...? body",),
                snippet="Variables are assigned from list elements; `body` runs once per assignment group.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="foreach varList list ?varList list ...? body",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(3),
            ),
            wasm_emits_nothing=True,
            arg_role_resolver=_foreach_arg_roles,
            return_type=TclType.STRING,
            arg_type_resolver=resolve_foreach,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
