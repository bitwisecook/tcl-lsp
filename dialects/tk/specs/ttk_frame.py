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

"""ttk::frame -- Themed frame container widget."""

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

_SOURCE = "Tk man page ttk_frame.n"


@register
class TtkFrameCommand(CommandDef):
    name = "ttk::frame"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ttk::frame",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a themed frame container widget.",
                synopsis=("ttk::frame pathName ?options?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ttk::frame pathName ?options?",
                    options=(
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            value_hint="width",
                            detail="Desired width of the frame.",
                        ),
                        OptionSpec(
                            name="-height",
                            takes_value=True,
                            value_hint="height",
                            detail="Desired height of the frame.",
                        ),
                        OptionSpec(
                            name="-relief",
                            takes_value=True,
                            value_hint="relief",
                            detail="Border relief style for the frame.",
                        ),
                        OptionSpec(
                            name="-borderwidth",
                            takes_value=True,
                            value_hint="width",
                            detail="Width of the frame border.",
                        ),
                        OptionSpec(
                            name="-padding",
                            takes_value=True,
                            value_hint="padSpec",
                            detail="Internal padding around the frame content.",
                        ),
                        OptionSpec(
                            name="-style",
                            takes_value=True,
                            value_hint="style",
                            detail="Style to use for the widget.",
                        ),
                        OptionSpec(
                            name="-class",
                            takes_value=True,
                            value_hint="className",
                            detail="Widget class name for option-database lookups.",
                        ),
                        OptionSpec(
                            name="-cursor",
                            takes_value=True,
                            value_hint="cursor",
                            detail="Cursor to display when the pointer is over the widget.",
                        ),
                        OptionSpec(
                            name="-takefocus",
                            takes_value=True,
                            value_hint="focusSpec",
                            detail="Whether the widget accepts focus during keyboard traversal.",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
