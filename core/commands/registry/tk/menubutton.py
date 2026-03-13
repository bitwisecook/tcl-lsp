"""menubutton -- Create and manipulate menubutton widgets."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page menubutton.n"


@register
class MenubuttonCommand(CommandDef):
    name = "menubutton"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="menubutton",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a menubutton widget.",
                synopsis=("menubutton pathName ?option value ...?",),
                snippet=(
                    "Displays a textual string, bitmap, or image. "
                    "When pressed, posts an associated menu widget."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="menubutton pathName ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-text",
                            takes_value=True,
                            detail="Text to display in the menubutton.",
                        ),
                        OptionSpec(
                            name="-textvariable",
                            takes_value=True,
                            detail="Name of a variable whose value will be used as the menubutton text.",
                        ),
                        OptionSpec(
                            name="-menu",
                            takes_value=True,
                            detail="Path name of the menu widget to post when the menubutton is pressed.",
                        ),
                        OptionSpec(
                            name="-direction",
                            takes_value=True,
                            detail="Direction to post the menu: above, below, flush, left, or right.",
                        ),
                        OptionSpec(
                            name="-image",
                            takes_value=True,
                            detail="Image to display in the menubutton.",
                        ),
                        OptionSpec(
                            name="-bitmap",
                            takes_value=True,
                            detail="Bitmap to display in the menubutton.",
                        ),
                        OptionSpec(
                            name="-compound",
                            takes_value=True,
                            detail="Whether to display both image and text: none, bottom, top, left, right, or center.",
                        ),
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            detail="Desired width of the menubutton in characters (text) or pixels (image).",
                        ),
                        OptionSpec(
                            name="-height",
                            takes_value=True,
                            detail="Desired height of the menubutton in lines (text) or pixels (image).",
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
                            name="-state",
                            takes_value=True,
                            detail="State of the menubutton: normal, active, or disabled.",
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
                            detail="Font to use for the menubutton text.",
                        ),
                        OptionSpec(
                            name="-padx",
                            takes_value=True,
                            detail="Extra horizontal padding inside the menubutton.",
                        ),
                        OptionSpec(
                            name="-pady",
                            takes_value=True,
                            detail="Extra vertical padding inside the menubutton.",
                        ),
                        OptionSpec(
                            name="-activebackground",
                            takes_value=True,
                            detail="Background colour when the menubutton is active (mouse over).",
                        ),
                        OptionSpec(
                            name="-activeforeground",
                            takes_value=True,
                            detail="Foreground colour when the menubutton is active.",
                        ),
                        OptionSpec(
                            name="-disabledforeground",
                            takes_value=True,
                            detail="Foreground colour when the menubutton is disabled.",
                        ),
                        OptionSpec(
                            name="-highlightbackground",
                            takes_value=True,
                            detail="Colour of the highlight region when the menubutton does not have focus.",
                        ),
                        OptionSpec(
                            name="-highlightcolor",
                            takes_value=True,
                            detail="Colour of the highlight region when the menubutton has focus.",
                        ),
                        OptionSpec(
                            name="-highlightthickness",
                            takes_value=True,
                            detail="Width of the highlight rectangle drawn around the menubutton.",
                        ),
                        OptionSpec(
                            name="-cursor",
                            takes_value=True,
                            detail="Cursor to display when the mouse is over the menubutton.",
                        ),
                        OptionSpec(
                            name="-takefocus",
                            takes_value=True,
                            detail="Whether the menubutton accepts focus during keyboard traversal.",
                        ),
                        OptionSpec(
                            name="-indicatoron",
                            takes_value=True,
                            detail="Whether to display a small indicator showing the menu direction.",
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
