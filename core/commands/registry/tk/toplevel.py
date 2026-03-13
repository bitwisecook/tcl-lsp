"""toplevel -- Create and manipulate toplevel widgets."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page toplevel.n"


@register
class ToplevelCommand(CommandDef):
    name = "toplevel"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="toplevel",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a toplevel widget.",
                synopsis=("toplevel pathName ?option value ...?",),
                snippet=(
                    "Creates a new toplevel window that acts as a separate "
                    "window manager frame, suitable for dialogue boxes and secondary windows."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="toplevel pathName ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            detail="Desired width of the toplevel in screen units.",
                        ),
                        OptionSpec(
                            name="-height",
                            takes_value=True,
                            detail="Desired height of the toplevel in screen units.",
                        ),
                        OptionSpec(
                            name="-bg",
                            takes_value=True,
                            detail="Shorthand for -background.",
                        ),
                        OptionSpec(
                            name="-background",
                            takes_value=True,
                            detail="Background colour of the toplevel window.",
                        ),
                        OptionSpec(
                            name="-relief",
                            takes_value=True,
                            detail="3-D effect: flat, groove, raised, ridge, solid, or sunken.",
                        ),
                        OptionSpec(
                            name="-borderwidth",
                            takes_value=True,
                            detail="Width of the border around the toplevel.",
                        ),
                        OptionSpec(
                            name="-menu",
                            takes_value=True,
                            detail="Path name of a menu widget to use as the toplevel's menu bar.",
                        ),
                        OptionSpec(
                            name="-screen",
                            takes_value=True,
                            detail="Screen on which to place the toplevel window.",
                        ),
                        OptionSpec(
                            name="-use",
                            takes_value=True,
                            detail="Window identifier of a container in which to embed the toplevel.",
                        ),
                        OptionSpec(
                            name="-class",
                            takes_value=True,
                            detail="Class name for the toplevel, used in option database lookups.",
                        ),
                        OptionSpec(
                            name="-colormap",
                            takes_value=True,
                            detail="Colourmap to use for the toplevel: new or inherited from a window.",
                        ),
                        OptionSpec(
                            name="-container",
                            takes_value=True,
                            detail="Whether the toplevel will be a container for an embedded application.",
                        ),
                        OptionSpec(
                            name="-visual",
                            takes_value=True,
                            detail="Visual information for the toplevel window.",
                        ),
                        OptionSpec(
                            name="-cursor",
                            takes_value=True,
                            detail="Cursor to display when the mouse is over the toplevel.",
                        ),
                        OptionSpec(
                            name="-takefocus",
                            takes_value=True,
                            detail="Whether the toplevel accepts focus during keyboard traversal.",
                        ),
                        OptionSpec(
                            name="-highlightbackground",
                            takes_value=True,
                            detail="Colour of the highlight region when the toplevel does not have focus.",
                        ),
                        OptionSpec(
                            name="-highlightcolor",
                            takes_value=True,
                            detail="Colour of the highlight region when the toplevel has focus.",
                        ),
                        OptionSpec(
                            name="-highlightthickness",
                            takes_value=True,
                            detail="Width of the highlight rectangle drawn around the toplevel.",
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
