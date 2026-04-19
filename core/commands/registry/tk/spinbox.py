"""spinbox -- Create and manipulate spinbox widgets."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page spinbox.n"


@register
class SpinboxCommand(CommandDef):
    name = "spinbox"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="spinbox",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a spinbox widget.",
                synopsis=("spinbox pathName ?option value ...?",),
                snippet=(
                    "Displays a single-line text field with increment and "
                    "decrement arrows for cycling through a range of values."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="spinbox pathName ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-from",
                            takes_value=True,
                            detail="Starting value for the numeric range.",
                        ),
                        OptionSpec(
                            name="-to",
                            takes_value=True,
                            detail="Ending value for the numeric range.",
                        ),
                        OptionSpec(
                            name="-increment",
                            takes_value=True,
                            detail="Amount to increment or decrement the value on each arrow press.",
                        ),
                        OptionSpec(
                            name="-values",
                            takes_value=True,
                            detail="List of values to cycle through instead of a numeric range.",
                        ),
                        OptionSpec(
                            name="-textvariable",
                            takes_value=True,
                            detail="Name of a variable linked to the spinbox's contents.",
                        ),
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            detail="Desired width of the spinbox in average-size characters.",
                        ),
                        OptionSpec(
                            name="-state",
                            takes_value=True,
                            detail="State of the spinbox: normal, disabled, or readonly.",
                        ),
                        OptionSpec(
                            name="-format",
                            takes_value=True,
                            detail="Format string for displaying the value (e.g. %5.2f).",
                        ),
                        OptionSpec(
                            name="-wrap",
                            takes_value=True,
                            detail="Whether the value wraps around when the range limit is reached.",
                        ),
                        OptionSpec(
                            name="-command",
                            takes_value=True,
                            detail="Tcl command to invoke when the value is changed via the arrows.",
                        ),
                        OptionSpec(
                            name="-validate",
                            takes_value=True,
                            detail="Validation mode: none, focus, focusin, focusout, key, or all.",
                        ),
                        OptionSpec(
                            name="-validatecommand",
                            takes_value=True,
                            detail="Script to evaluate when validation is triggered.",
                        ),
                        OptionSpec(
                            name="-invalidcommand",
                            takes_value=True,
                            detail="Script to evaluate when validation fails.",
                        ),
                        OptionSpec(
                            name="-font",
                            takes_value=True,
                            detail="Font to use for text in the spinbox.",
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
                            name="-readonlybackground",
                            takes_value=True,
                            detail="Background colour when the spinbox is in readonly state.",
                        ),
                        OptionSpec(
                            name="-buttonbackground",
                            takes_value=True,
                            detail="Background colour of the increment/decrement buttons.",
                        ),
                        OptionSpec(
                            name="-buttoncursor",
                            takes_value=True,
                            detail="Cursor to display when the mouse is over the buttons.",
                        ),
                        OptionSpec(
                            name="-buttondownrelief",
                            takes_value=True,
                            detail="Relief of the down (decrement) button.",
                        ),
                        OptionSpec(
                            name="-buttonuprelief",
                            takes_value=True,
                            detail="Relief of the up (increment) button.",
                        ),
                        OptionSpec(
                            name="-relief",
                            takes_value=True,
                            detail="3-D effect: flat, groove, raised, ridge, solid, or sunken.",
                        ),
                        OptionSpec(
                            name="-selectbackground",
                            takes_value=True,
                            detail="Background colour for selected text.",
                        ),
                        OptionSpec(
                            name="-selectborderwidth",
                            takes_value=True,
                            detail="Width of the border around selected text.",
                        ),
                        OptionSpec(
                            name="-selectforeground",
                            takes_value=True,
                            detail="Foreground colour for selected text.",
                        ),
                        OptionSpec(
                            name="-insertbackground",
                            takes_value=True,
                            detail="Colour of the insertion cursor.",
                        ),
                        OptionSpec(
                            name="-insertborderwidth",
                            takes_value=True,
                            detail="Width of the border around the insertion cursor.",
                        ),
                        OptionSpec(
                            name="-insertofftime",
                            takes_value=True,
                            detail="Milliseconds the insertion cursor is off during blinking.",
                        ),
                        OptionSpec(
                            name="-insertontime",
                            takes_value=True,
                            detail="Milliseconds the insertion cursor is on during blinking.",
                        ),
                        OptionSpec(
                            name="-insertwidth",
                            takes_value=True,
                            detail="Width of the insertion cursor in screen units.",
                        ),
                        OptionSpec(
                            name="-xscrollcommand",
                            takes_value=True,
                            detail="Command prefix for communicating with horizontal scrollbars.",
                        ),
                        OptionSpec(
                            name="-exportselection",
                            takes_value=True,
                            detail="Whether the selection is exported to the X selection.",
                        ),
                        OptionSpec(
                            name="-cursor",
                            takes_value=True,
                            detail="Cursor to display when the mouse is over the spinbox.",
                        ),
                        OptionSpec(
                            name="-takefocus",
                            takes_value=True,
                            detail="Whether the spinbox accepts focus during keyboard traversal.",
                        ),
                        OptionSpec(
                            name="-highlightbackground",
                            takes_value=True,
                            detail="Colour of the highlight region when the spinbox does not have focus.",
                        ),
                        OptionSpec(
                            name="-highlightcolor",
                            takes_value=True,
                            detail="Colour of the highlight region when the spinbox has focus.",
                        ),
                        OptionSpec(
                            name="-highlightthickness",
                            takes_value=True,
                            detail="Width of the highlight rectangle drawn around the spinbox.",
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
