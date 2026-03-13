"""frame -- Create and manipulate frame widgets."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page frame.n"


@register
class FrameCommand(CommandDef):
    name = "frame"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="frame",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a frame widget.",
                synopsis=("frame pathName ?option value ...?",),
                snippet=(
                    "A frame is a simple container widget used primarily to "
                    "group and organise other widgets."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="frame pathName ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            detail="Desired width of the frame in screen units.",
                        ),
                        OptionSpec(
                            name="-height",
                            takes_value=True,
                            detail="Desired height of the frame in screen units.",
                        ),
                        OptionSpec(
                            name="-relief",
                            takes_value=True,
                            detail="3-D effect: flat, groove, raised, ridge, solid, or sunken.",
                        ),
                        OptionSpec(
                            name="-borderwidth",
                            takes_value=True,
                            detail="Width of the border around the frame.",
                        ),
                        OptionSpec(
                            name="-bd",
                            takes_value=True,
                            detail="Shorthand for -borderwidth.",
                        ),
                        OptionSpec(
                            name="-bg",
                            takes_value=True,
                            detail="Shorthand for -background.",
                        ),
                        OptionSpec(
                            name="-background",
                            takes_value=True,
                            detail="Background colour of the frame.",
                        ),
                        OptionSpec(
                            name="-cursor",
                            takes_value=True,
                            detail="Cursor to display when the mouse is over the frame.",
                        ),
                        OptionSpec(
                            name="-takefocus",
                            takes_value=True,
                            detail="Whether the frame accepts focus during keyboard traversal.",
                        ),
                        OptionSpec(
                            name="-highlightbackground",
                            takes_value=True,
                            detail="Colour of the highlight region when the frame does not have focus.",
                        ),
                        OptionSpec(
                            name="-highlightcolor",
                            takes_value=True,
                            detail="Colour of the highlight region when the frame has focus.",
                        ),
                        OptionSpec(
                            name="-highlightthickness",
                            takes_value=True,
                            detail="Width of the highlight rectangle drawn around the frame.",
                        ),
                        OptionSpec(
                            name="-padx",
                            takes_value=True,
                            detail="Extra horizontal padding inside the frame.",
                        ),
                        OptionSpec(
                            name="-pady",
                            takes_value=True,
                            detail="Extra vertical padding inside the frame.",
                        ),
                        OptionSpec(
                            name="-class",
                            takes_value=True,
                            detail="Class name for the frame, used in option database lookups.",
                        ),
                        OptionSpec(
                            name="-colormap",
                            takes_value=True,
                            detail="Colourmap to use for the frame: new or inherited from a window.",
                        ),
                        OptionSpec(
                            name="-container",
                            takes_value=True,
                            detail="Whether the frame will be a container for an embedded application.",
                        ),
                        OptionSpec(
                            name="-visual",
                            takes_value=True,
                            detail="Visual information for the frame.",
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
