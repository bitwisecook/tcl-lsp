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

"""tk_chooseColor -- Display a colour chooser dialogue."""

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
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import register

_SOURCE = "Tk man page tk_chooseColor.n"


@register
class TkChooseColorCommand(CommandDef):
    name = "tk_chooseColor"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tk_chooseColor",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Pop up a dialogue for the user to select a colour.",
                synopsis=("tk_chooseColor ?option value ...?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="tk_chooseColor ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-initialcolor",
                            takes_value=True,
                            value_hint="colour",
                            detail="Initial colour to display in the chooser.",
                        ),
                        OptionSpec(
                            name="-parent",
                            takes_value=True,
                            value_hint="window",
                            detail="Parent window for the dialogue.",
                        ),
                        OptionSpec(
                            name="-title",
                            takes_value=True,
                            value_hint="titleString",
                            detail="Title string for the dialogue window.",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0),
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
