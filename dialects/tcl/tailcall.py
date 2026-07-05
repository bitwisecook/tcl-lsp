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

# Scaffolded from tailcall.n -- refine and commit
"""tailcall -- Replace the current procedure with another command."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page tailcall.n"


@register
class TailcallCommand(CommandDef):
    name = "tailcall"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tailcall",
            byte_compiled=True,
            is_language_keyword=True,
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="Replace the current procedure with another command",
                synopsis=("tailcall command ?arg ...?",),
                snippet="The tailcall command replaces the currently executing procedure, lambda application, or method with another command.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="tailcall command ?arg ...?",
                ),
            ),
            validation=ValidationSpec(
                # C Tcl 9.0 ``TclNRTailcallObjCmd``: ``if (objc < 1)``
                # is unreachable, so any arg count is accepted.
                # Invocation without args clears a previously
                # scheduled tailcall; with args it replaces it.  So
                # the real arity is 0..∞.
                arity=Arity(0),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE, connection_side=ConnectionSide.NONE
                ),
            ),
        )
