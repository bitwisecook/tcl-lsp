"""scale -- Create and manipulate scale (slider) widgets."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page scale.n"


@register
class ScaleCommand(CommandDef):
    name = "scale"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="scale",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a scale (slider) widget.",
                synopsis=("scale pathName ?option value ...?",),
                snippet=(
                    "Displays a slider that allows the user to select "
                    "a numerical value from a specified range."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="scale pathName ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-from",
                            takes_value=True,
                            detail="Starting value of the range (a real number).",
                        ),
                        OptionSpec(
                            name="-to",
                            takes_value=True,
                            detail="Ending value of the range (a real number).",
                        ),
                        OptionSpec(
                            name="-variable",
                            takes_value=True,
                            detail="Name of a variable linked to the scale's current value.",
                        ),
                        OptionSpec(
                            name="-orient",
                            takes_value=True,
                            detail="Orientation of the scale: horizontal or vertical.",
                        ),
                        OptionSpec(
                            name="-resolution",
                            takes_value=True,
                            detail="Resolution (step size) for the scale value.",
                        ),
                        OptionSpec(
                            name="-tickinterval",
                            takes_value=True,
                            detail="Spacing between numerical tick marks displayed along the scale.",
                        ),
                        OptionSpec(
                            name="-label",
                            takes_value=True,
                            detail="Text label to display alongside the scale.",
                        ),
                        OptionSpec(
                            name="-length",
                            takes_value=True,
                            detail="Desired long dimension of the scale in screen units.",
                        ),
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            detail="Desired narrow dimension of the trough in screen units.",
                        ),
                        OptionSpec(
                            name="-sliderlength",
                            takes_value=True,
                            detail="Length of the slider along the long dimension in screen units.",
                        ),
                        OptionSpec(
                            name="-sliderrelief",
                            takes_value=True,
                            detail="Relief of the slider: flat, groove, raised, ridge, solid, or sunken.",
                        ),
                        OptionSpec(
                            name="-showvalue",
                            takes_value=True,
                            detail="Whether to display the current value next to the slider.",
                        ),
                        OptionSpec(
                            name="-digits",
                            takes_value=True,
                            detail="Number of significant digits for the scale value.",
                        ),
                        OptionSpec(
                            name="-bigincrement",
                            takes_value=True,
                            detail="Large increment used for Control-arrow key bindings.",
                        ),
                        OptionSpec(
                            name="-command",
                            takes_value=True,
                            detail="Tcl command prefix invoked when the scale value changes.",
                        ),
                        OptionSpec(
                            name="-state",
                            takes_value=True,
                            detail="State of the scale: normal, active, or disabled.",
                        ),
                        OptionSpec(
                            name="-font",
                            takes_value=True,
                            detail="Font to use for the label and value display.",
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
                            name="-troughcolor",
                            takes_value=True,
                            detail="Colour of the trough area.",
                        ),
                        OptionSpec(
                            name="-activebackground",
                            takes_value=True,
                            detail="Background colour when the slider is active (mouse over).",
                        ),
                        OptionSpec(
                            name="-highlightbackground",
                            takes_value=True,
                            detail="Colour of the highlight region when the scale does not have focus.",
                        ),
                        OptionSpec(
                            name="-highlightcolor",
                            takes_value=True,
                            detail="Colour of the highlight region when the scale has focus.",
                        ),
                        OptionSpec(
                            name="-highlightthickness",
                            takes_value=True,
                            detail="Width of the highlight rectangle drawn around the scale.",
                        ),
                        OptionSpec(
                            name="-relief",
                            takes_value=True,
                            detail="3-D effect: flat, groove, raised, ridge, solid, or sunken.",
                        ),
                        OptionSpec(
                            name="-borderwidth",
                            takes_value=True,
                            detail="Width of the border around the scale.",
                        ),
                        OptionSpec(
                            name="-cursor",
                            takes_value=True,
                            detail="Cursor to display when the mouse is over the scale.",
                        ),
                        OptionSpec(
                            name="-takefocus",
                            takes_value=True,
                            detail="Whether the scale accepts focus during keyboard traversal.",
                        ),
                        OptionSpec(
                            name="-repeatdelay",
                            takes_value=True,
                            detail="Milliseconds before auto-repeat begins when trough is held.",
                        ),
                        OptionSpec(
                            name="-repeatinterval",
                            takes_value=True,
                            detail="Milliseconds between auto-repeat invocations.",
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
