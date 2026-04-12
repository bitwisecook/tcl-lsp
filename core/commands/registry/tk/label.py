"""label -- Create and manipulate label widgets."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page label.n"


@register
class LabelCommand(CommandDef):
    name = "label"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="label",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a label widget.",
                synopsis=("label pathName ?option value ...?",),
                snippet=(
                    "Displays a textual string, bitmap, or image. "
                    "A label is a non-interactive widget."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="label pathName ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-text",
                            takes_value=True,
                            detail="Text string to be displayed in the label.",
                        ),
                        OptionSpec(
                            name="-textvariable",
                            takes_value=True,
                            detail="Name of a variable whose value will be used as the label text.",
                        ),
                        OptionSpec(
                            name="-image",
                            takes_value=True,
                            detail="Image to display in the label.",
                        ),
                        OptionSpec(
                            name="-bitmap",
                            takes_value=True,
                            detail="Bitmap to display in the label.",
                        ),
                        OptionSpec(
                            name="-compound",
                            takes_value=True,
                            detail="Whether to display both image and text: none, bottom, top, left, right, or center.",
                        ),
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            detail="Desired width of the label in characters (text) or pixels (image).",
                        ),
                        OptionSpec(
                            name="-height",
                            takes_value=True,
                            detail="Desired height of the label in lines (text) or pixels (image).",
                        ),
                        OptionSpec(
                            name="-anchor",
                            takes_value=True,
                            detail="How information is positioned: n, ne, e, se, s, sw, w, nw, or center.",
                        ),
                        OptionSpec(
                            name="-justify",
                            takes_value=True,
                            detail="Justification of multi-line text: left, center, or right.",
                        ),
                        OptionSpec(
                            name="-wraplength",
                            takes_value=True,
                            detail="Maximum line length for word wrapping, in screen units.",
                        ),
                        OptionSpec(
                            name="-relief",
                            takes_value=True,
                            detail="3-D effect: flat, groove, raised, ridge, solid, or sunken.",
                        ),
                        OptionSpec(
                            name="-bg",
                            takes_value=True,
                            detail="Shorthand for -background.",
                        ),
                        OptionSpec(
                            name="-fg",
                            takes_value=True,
                            detail="Shorthand for -foreground.",
                        ),
                        OptionSpec(
                            name="-font",
                            takes_value=True,
                            detail="Font to use for the label text.",
                        ),
                        OptionSpec(
                            name="-padx",
                            takes_value=True,
                            detail="Extra horizontal padding inside the label.",
                        ),
                        OptionSpec(
                            name="-pady",
                            takes_value=True,
                            detail="Extra vertical padding inside the label.",
                        ),
                        OptionSpec(
                            name="-cursor",
                            takes_value=True,
                            detail="Cursor to display when the mouse is over the label.",
                        ),
                        OptionSpec(
                            name="-takefocus",
                            takes_value=True,
                            detail="Whether the label accepts focus during keyboard traversal.",
                        ),
                        OptionSpec(
                            name="-underline",
                            takes_value=True,
                            detail="Index of character to underline for keyboard traversal (0-based).",
                        ),
                        OptionSpec(
                            name="-highlightbackground",
                            takes_value=True,
                            detail="Colour of the highlight region when the label does not have focus.",
                        ),
                        OptionSpec(
                            name="-highlightcolor",
                            takes_value=True,
                            detail="Colour of the highlight region when the label has focus.",
                        ),
                        OptionSpec(
                            name="-highlightthickness",
                            takes_value=True,
                            detail="Width of the highlight rectangle drawn around the label.",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
