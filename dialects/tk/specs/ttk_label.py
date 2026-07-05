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

"""ttk::label -- Themed label widget."""

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

_SOURCE = "Tk man page ttk_label.n"


@register
class TtkLabelCommand(CommandDef):
    name = "ttk::label"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ttk::label",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a themed label widget.",
                synopsis=("ttk::label pathName ?options?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ttk::label pathName ?options?",
                    options=(
                        OptionSpec(
                            name="-text",
                            takes_value=True,
                            value_hint="string",
                            detail="Text to display in the label.",
                        ),
                        OptionSpec(
                            name="-textvariable",
                            takes_value=True,
                            value_hint="varName",
                            detail="Variable whose value is used as the label text.",
                        ),
                        OptionSpec(
                            name="-image",
                            takes_value=True,
                            value_hint="imageName",
                            detail="Image to display in the label.",
                        ),
                        OptionSpec(
                            name="-compound",
                            takes_value=True,
                            value_hint="compoundType",
                            detail="How to display image relative to text.",
                        ),
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            value_hint="width",
                            detail="Desired width of the label.",
                        ),
                        OptionSpec(
                            name="-anchor",
                            takes_value=True,
                            value_hint="anchorPos",
                            detail="How the text or image is positioned within the widget.",
                        ),
                        OptionSpec(
                            name="-justify",
                            takes_value=True,
                            value_hint="justification",
                            detail="How to justify multiple lines of text.",
                        ),
                        OptionSpec(
                            name="-wraplength",
                            takes_value=True,
                            value_hint="length",
                            detail="Maximum line length for word wrapping.",
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
                        OptionSpec(
                            name="-padding",
                            takes_value=True,
                            value_hint="padSpec",
                            detail="Internal padding around the widget content.",
                        ),
                        OptionSpec(
                            name="-underline",
                            takes_value=True,
                            value_hint="index",
                            detail="Index of the character to underline for mnemonic activation.",
                        ),
                        OptionSpec(
                            name="-relief",
                            takes_value=True,
                            value_hint="relief",
                            detail="Border relief style for the label.",
                        ),
                        OptionSpec(
                            name="-font",
                            takes_value=True,
                            value_hint="font",
                            detail="Font to use for the label text.",
                        ),
                        OptionSpec(
                            name="-foreground",
                            takes_value=True,
                            value_hint="colour",
                            detail="Foreground colour for the label text.",
                        ),
                        OptionSpec(
                            name="-background",
                            takes_value=True,
                            value_hint="colour",
                            detail="Background colour for the label.",
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
