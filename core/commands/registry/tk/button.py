"""button -- Create and manipulate button widgets."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page button.n"


@register
class ButtonCommand(CommandDef):
    name = "button"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="button",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a button widget.",
                synopsis=("button pathName ?option value ...?",),
                snippet=(
                    "Displays a textual string, bitmap, or image. "
                    "When pressed, invokes a Tcl command."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="button pathName ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-text",
                            takes_value=True,
                            detail="Text to display in the button.",
                        ),
                        OptionSpec(
                            name="-textvariable",
                            takes_value=True,
                            detail="Name of a variable whose value will be used as the button text.",
                        ),
                        OptionSpec(
                            name="-command",
                            takes_value=True,
                            detail="Tcl command to invoke when the button is pressed.",
                        ),
                        OptionSpec(
                            name="-state",
                            takes_value=True,
                            detail="State of the button: normal, active, or disabled.",
                        ),
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            detail="Desired width of the button in characters (text) or pixels (image).",
                        ),
                        OptionSpec(
                            name="-height",
                            takes_value=True,
                            detail="Desired height of the button in lines (text) or pixels (image).",
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
                            name="-background",
                            takes_value=True,
                            detail="Normal background colour of the button.",
                        ),
                        OptionSpec(
                            name="-fg",
                            takes_value=True,
                            detail="Shorthand for -foreground.",
                        ),
                        OptionSpec(
                            name="-foreground",
                            takes_value=True,
                            detail="Normal foreground colour of the button.",
                        ),
                        OptionSpec(
                            name="-font",
                            takes_value=True,
                            detail="Font to use for the button text.",
                        ),
                        OptionSpec(
                            name="-image",
                            takes_value=True,
                            detail="Image to display in the button.",
                        ),
                        OptionSpec(
                            name="-bitmap",
                            takes_value=True,
                            detail="Bitmap to display in the button.",
                        ),
                        OptionSpec(
                            name="-compound",
                            takes_value=True,
                            detail="Whether to display both image and text: none, bottom, top, left, right, or center.",
                        ),
                        OptionSpec(
                            name="-padx",
                            takes_value=True,
                            detail="Extra horizontal padding inside the button.",
                        ),
                        OptionSpec(
                            name="-pady",
                            takes_value=True,
                            detail="Extra vertical padding inside the button.",
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
                            name="-underline",
                            takes_value=True,
                            detail="Index of character to underline for keyboard traversal (0-based).",
                        ),
                        OptionSpec(
                            name="-activebackground",
                            takes_value=True,
                            detail="Background colour when the button is active (mouse over).",
                        ),
                        OptionSpec(
                            name="-activeforeground",
                            takes_value=True,
                            detail="Foreground colour when the button is active.",
                        ),
                        OptionSpec(
                            name="-disabledforeground",
                            takes_value=True,
                            detail="Foreground colour when the button is disabled.",
                        ),
                        OptionSpec(
                            name="-highlightbackground",
                            takes_value=True,
                            detail="Colour of the highlight region when the button does not have focus.",
                        ),
                        OptionSpec(
                            name="-highlightcolor",
                            takes_value=True,
                            detail="Colour of the highlight region when the button has focus.",
                        ),
                        OptionSpec(
                            name="-highlightthickness",
                            takes_value=True,
                            detail="Width of the highlight rectangle drawn around the button.",
                        ),
                        OptionSpec(
                            name="-cursor",
                            takes_value=True,
                            detail="Cursor to display when the mouse is over the button.",
                        ),
                        OptionSpec(
                            name="-takefocus",
                            takes_value=True,
                            detail="Whether the button accepts focus during keyboard traversal.",
                        ),
                        OptionSpec(
                            name="-repeatdelay",
                            takes_value=True,
                            detail="Milliseconds before auto-repeat begins when button is held.",
                        ),
                        OptionSpec(
                            name="-repeatinterval",
                            takes_value=True,
                            detail="Milliseconds between auto-repeat invocations.",
                        ),
                        OptionSpec(
                            name="-overrelief",
                            takes_value=True,
                            detail="Relief to use when the mouse cursor is over the button.",
                        ),
                        OptionSpec(
                            name="-default",
                            takes_value=True,
                            detail="Default ring state: normal, active, or disabled.",
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
