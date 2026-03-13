"""radiobutton -- Create and manipulate radiobutton widgets."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page radiobutton.n"


@register
class RadiobuttonCommand(CommandDef):
    name = "radiobutton"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="radiobutton",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a radiobutton widget.",
                synopsis=("radiobutton pathName ?option value ...?",),
                snippet=(
                    "Displays a textual string, bitmap, or image with a "
                    "selection indicator for mutual-exclusion choices within a group."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="radiobutton pathName ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-text",
                            takes_value=True,
                            detail="Text to display in the radiobutton.",
                        ),
                        OptionSpec(
                            name="-textvariable",
                            takes_value=True,
                            detail="Name of a variable whose value will be used as the radiobutton text.",
                        ),
                        OptionSpec(
                            name="-variable",
                            takes_value=True,
                            detail="Name of the global variable shared by all radiobuttons in the group.",
                        ),
                        OptionSpec(
                            name="-value",
                            takes_value=True,
                            detail="Value stored in the variable when this radiobutton is selected.",
                        ),
                        OptionSpec(
                            name="-command",
                            takes_value=True,
                            detail="Tcl command to invoke when the radiobutton is selected.",
                        ),
                        OptionSpec(
                            name="-state",
                            takes_value=True,
                            detail="State of the radiobutton: normal, active, or disabled.",
                        ),
                        OptionSpec(
                            name="-indicatoron",
                            takes_value=True,
                            detail="Whether to display the selection indicator.",
                        ),
                        OptionSpec(
                            name="-selectimage",
                            takes_value=True,
                            detail="Image to display when the radiobutton is selected.",
                        ),
                        OptionSpec(
                            name="-selectcolor",
                            takes_value=True,
                            detail="Colour of the indicator when the radiobutton is selected.",
                        ),
                        OptionSpec(
                            name="-image",
                            takes_value=True,
                            detail="Image to display in the radiobutton.",
                        ),
                        OptionSpec(
                            name="-bitmap",
                            takes_value=True,
                            detail="Bitmap to display in the radiobutton.",
                        ),
                        OptionSpec(
                            name="-compound",
                            takes_value=True,
                            detail="Whether to display both image and text: none, bottom, top, left, right, or center.",
                        ),
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            detail="Desired width of the radiobutton in characters (text) or pixels (image).",
                        ),
                        OptionSpec(
                            name="-height",
                            takes_value=True,
                            detail="Desired height of the radiobutton in lines (text) or pixels (image).",
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
                            name="-font",
                            takes_value=True,
                            detail="Font to use for the radiobutton text.",
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
                            name="-activebackground",
                            takes_value=True,
                            detail="Background colour when the radiobutton is active (mouse over).",
                        ),
                        OptionSpec(
                            name="-activeforeground",
                            takes_value=True,
                            detail="Foreground colour when the radiobutton is active.",
                        ),
                        OptionSpec(
                            name="-disabledforeground",
                            takes_value=True,
                            detail="Foreground colour when the radiobutton is disabled.",
                        ),
                        OptionSpec(
                            name="-relief",
                            takes_value=True,
                            detail="3-D effect: flat, groove, raised, ridge, solid, or sunken.",
                        ),
                        OptionSpec(
                            name="-padx",
                            takes_value=True,
                            detail="Extra horizontal padding inside the radiobutton.",
                        ),
                        OptionSpec(
                            name="-pady",
                            takes_value=True,
                            detail="Extra vertical padding inside the radiobutton.",
                        ),
                        OptionSpec(
                            name="-cursor",
                            takes_value=True,
                            detail="Cursor to display when the mouse is over the radiobutton.",
                        ),
                        OptionSpec(
                            name="-takefocus",
                            takes_value=True,
                            detail="Whether the radiobutton accepts focus during keyboard traversal.",
                        ),
                        OptionSpec(
                            name="-highlightbackground",
                            takes_value=True,
                            detail="Colour of the highlight region when the radiobutton does not have focus.",
                        ),
                        OptionSpec(
                            name="-highlightcolor",
                            takes_value=True,
                            detail="Colour of the highlight region when the radiobutton has focus.",
                        ),
                        OptionSpec(
                            name="-highlightthickness",
                            takes_value=True,
                            detail="Width of the highlight rectangle drawn around the radiobutton.",
                        ),
                        OptionSpec(
                            name="-overrelief",
                            takes_value=True,
                            detail="Relief to use when the mouse cursor is over the radiobutton.",
                        ),
                        OptionSpec(
                            name="-tristateimage",
                            takes_value=True,
                            detail="Image to display when the radiobutton is in the tristate mode.",
                        ),
                        OptionSpec(
                            name="-tristatevalue",
                            takes_value=True,
                            detail="Value that places the radiobutton in tristate mode.",
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
