"""scrollbar -- Create and manipulate scrollbar widgets."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page scrollbar.n"


@register
class ScrollbarCommand(CommandDef):
    name = "scrollbar"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="scrollbar",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a scrollbar widget.",
                synopsis=("scrollbar pathName ?option value ...?",),
                snippet=(
                    "Displays a scrollbar and allows the user to "
                    "control the viewing area of an associated widget."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="scrollbar pathName ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-orient",
                            takes_value=True,
                            detail="Orientation of the scrollbar: horizontal or vertical.",
                        ),
                        OptionSpec(
                            name="-command",
                            takes_value=True,
                            detail="Command prefix to invoke when the scrollbar is moved.",
                        ),
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            detail="Desired narrow dimension of the scrollbar in screen units.",
                        ),
                        OptionSpec(
                            name="-bg",
                            takes_value=True,
                            detail="Shorthand for -background.",
                        ),
                        OptionSpec(
                            name="-background",
                            takes_value=True,
                            detail="Background colour of the scrollbar.",
                        ),
                        OptionSpec(
                            name="-activebackground",
                            takes_value=True,
                            detail="Background colour when the mouse is over the scrollbar elements.",
                        ),
                        OptionSpec(
                            name="-troughcolor",
                            takes_value=True,
                            detail="Colour of the trough area behind the slider.",
                        ),
                        OptionSpec(
                            name="-relief",
                            takes_value=True,
                            detail="3-D effect: flat, groove, raised, ridge, solid, or sunken.",
                        ),
                        OptionSpec(
                            name="-borderwidth",
                            takes_value=True,
                            detail="Width of the border around the scrollbar.",
                        ),
                        OptionSpec(
                            name="-elementborderwidth",
                            takes_value=True,
                            detail="Width of the borders around the internal elements of the scrollbar.",
                        ),
                        OptionSpec(
                            name="-jump",
                            takes_value=True,
                            detail="Whether to delay updates until the mouse button is released.",
                        ),
                        OptionSpec(
                            name="-activerelief",
                            takes_value=True,
                            detail="Relief to use for the active element of the scrollbar.",
                        ),
                        OptionSpec(
                            name="-cursor",
                            takes_value=True,
                            detail="Cursor to display when the mouse is over the scrollbar.",
                        ),
                        OptionSpec(
                            name="-takefocus",
                            takes_value=True,
                            detail="Whether the scrollbar accepts focus during keyboard traversal.",
                        ),
                        OptionSpec(
                            name="-highlightbackground",
                            takes_value=True,
                            detail="Colour of the highlight region when the scrollbar does not have focus.",
                        ),
                        OptionSpec(
                            name="-highlightcolor",
                            takes_value=True,
                            detail="Colour of the highlight region when the scrollbar has focus.",
                        ),
                        OptionSpec(
                            name="-highlightthickness",
                            takes_value=True,
                            detail="Width of the highlight rectangle drawn around the scrollbar.",
                        ),
                        OptionSpec(
                            name="-repeatdelay",
                            takes_value=True,
                            detail="Milliseconds before auto-repeat begins when an arrow is held.",
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
