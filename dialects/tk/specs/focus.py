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

"""focus -- Manage the input focus."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import register

_SOURCE = "Tk man page focus.n"
_av = make_av(_SOURCE)


@register
class FocusCommand(CommandDef):
    name = "focus"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="focus",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Manage the input focus.",
                synopsis=(
                    "focus",
                    "focus window",
                    "focus -displayof window",
                    "focus -force window",
                    "focus -lastfor window",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="focus ?option? ?window?",
                    options=(
                        OptionSpec(
                            name="-displayof",
                            takes_value=True,
                            value_hint="window",
                            detail="Return the focus window on the display of the given window.",
                        ),
                        OptionSpec(
                            name="-force",
                            takes_value=True,
                            value_hint="window",
                            detail="Set the focus to the window even if the application does not currently have focus.",
                        ),
                        OptionSpec(
                            name="-lastfor",
                            takes_value=True,
                            value_hint="window",
                            detail="Return the name of the most recent window to have the input focus among the window's top-level.",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0, 2),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
