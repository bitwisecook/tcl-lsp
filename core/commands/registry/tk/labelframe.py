"""labelframe -- Create and manipulate labelframe widgets."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page labelframe.n"


@register
class LabelframeCommand(CommandDef):
    name = "labelframe"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="labelframe",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a labelframe widget.",
                synopsis=("labelframe pathName ?option value ...?",),
                snippet=(
                    "Displays a frame with a decorative border and an "
                    "optional label, used to group related widgets visually."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="labelframe pathName ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-text",
                            takes_value=True,
                            detail="Text string to display as the label of the frame.",
                        ),
                        OptionSpec(
                            name="-labelanchor",
                            takes_value=True,
                            detail="Position of the label: nw, n, ne, en, e, es, se, s, sw, ws, w, or wn.",
                        ),
                        OptionSpec(
                            name="-labelwidget",
                            takes_value=True,
                            detail="Path name of a widget to use as the label instead of text.",
                        ),
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            detail="Desired width of the labelframe in screen units.",
                        ),
                        OptionSpec(
                            name="-height",
                            takes_value=True,
                            detail="Desired height of the labelframe in screen units.",
                        ),
                        OptionSpec(
                            name="-relief",
                            takes_value=True,
                            detail="3-D effect: flat, groove, raised, ridge, solid, or sunken.",
                        ),
                        OptionSpec(
                            name="-borderwidth",
                            takes_value=True,
                            detail="Width of the border around the labelframe.",
                        ),
                        OptionSpec(
                            name="-bg",
                            takes_value=True,
                            detail="Shorthand for -background.",
                        ),
                        OptionSpec(
                            name="-background",
                            takes_value=True,
                            detail="Background colour of the labelframe.",
                        ),
                        OptionSpec(
                            name="-fg",
                            takes_value=True,
                            detail="Shorthand for -foreground.",
                        ),
                        OptionSpec(
                            name="-foreground",
                            takes_value=True,
                            detail="Foreground colour for the label text.",
                        ),
                        OptionSpec(
                            name="-font",
                            takes_value=True,
                            detail="Font to use for the label text.",
                        ),
                        OptionSpec(
                            name="-padx",
                            takes_value=True,
                            detail="Extra horizontal padding inside the labelframe.",
                        ),
                        OptionSpec(
                            name="-pady",
                            takes_value=True,
                            detail="Extra vertical padding inside the labelframe.",
                        ),
                        OptionSpec(
                            name="-class",
                            takes_value=True,
                            detail="Class name for the labelframe, used in option database lookups.",
                        ),
                        OptionSpec(
                            name="-colormap",
                            takes_value=True,
                            detail="Colourmap to use for the labelframe: new or inherited from a window.",
                        ),
                        OptionSpec(
                            name="-container",
                            takes_value=True,
                            detail="Whether the labelframe will be a container for an embedded application.",
                        ),
                        OptionSpec(
                            name="-visual",
                            takes_value=True,
                            detail="Visual information for the labelframe.",
                        ),
                        OptionSpec(
                            name="-cursor",
                            takes_value=True,
                            detail="Cursor to display when the mouse is over the labelframe.",
                        ),
                        OptionSpec(
                            name="-takefocus",
                            takes_value=True,
                            detail="Whether the labelframe accepts focus during keyboard traversal.",
                        ),
                        OptionSpec(
                            name="-highlightbackground",
                            takes_value=True,
                            detail="Colour of the highlight region when the labelframe does not have focus.",
                        ),
                        OptionSpec(
                            name="-highlightcolor",
                            takes_value=True,
                            detail="Colour of the highlight region when the labelframe has focus.",
                        ),
                        OptionSpec(
                            name="-highlightthickness",
                            takes_value=True,
                            detail="Width of the highlight rectangle drawn around the labelframe.",
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
