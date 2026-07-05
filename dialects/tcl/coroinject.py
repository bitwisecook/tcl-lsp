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

# Based on the Tcl coroutine.n man page.
"""coroinject -- Inject a command into a suspended coroutine."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import register

_SOURCE = "Tcl man page coroutine.n"


@register
class CoroinjectCommand(CommandDef):
    name = "coroinject"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="coroinject",
            is_language_keyword=True,
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="Inject a command into a suspended coroutine.",
                synopsis=("coroinject coroName command ?arg ...?",),
                snippet="Arranges for the specified command to be executed inside the given coroutine the next time it resumes.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="coroinject coroName command ?arg ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
