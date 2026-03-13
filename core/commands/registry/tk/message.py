"""message -- Create and manipulate message widgets."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page message.n"


@register
class MessageCommand(CommandDef):
    name = "message"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="message",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a message widget.",
                synopsis=("message pathName ?option value ...?",),
                snippet=(
                    "Displays a multi-line text string using a single font. "
                    "Similar to a label but automatically wraps text to fit "
                    "a given aspect ratio or width."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="message pathName ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-text",
                            takes_value=True,
                            detail="Text string to be displayed in the message.",
                        ),
                        OptionSpec(
                            name="-textvariable",
                            takes_value=True,
                            detail="Name of a variable whose value will be used as the message text.",
                        ),
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            detail="Maximum line length for the message in screen units.",
                        ),
                        OptionSpec(
                            name="-aspect",
                            takes_value=True,
                            detail="Aspect ratio (100*width/height) for line wrapping; default is 150.",
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
                            name="-font",
                            takes_value=True,
                            detail="Font to use for the message text.",
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
                            name="-relief",
                            takes_value=True,
                            detail="3-D effect: flat, groove, raised, ridge, solid, or sunken.",
                        ),
                        OptionSpec(
                            name="-padx",
                            takes_value=True,
                            detail="Extra horizontal padding inside the message widget.",
                        ),
                        OptionSpec(
                            name="-pady",
                            takes_value=True,
                            detail="Extra vertical padding inside the message widget.",
                        ),
                        OptionSpec(
                            name="-cursor",
                            takes_value=True,
                            detail="Cursor to display when the mouse is over the message.",
                        ),
                        OptionSpec(
                            name="-takefocus",
                            takes_value=True,
                            detail="Whether the message accepts focus during keyboard traversal.",
                        ),
                        OptionSpec(
                            name="-highlightbackground",
                            takes_value=True,
                            detail="Colour of the highlight region when the message does not have focus.",
                        ),
                        OptionSpec(
                            name="-highlightcolor",
                            takes_value=True,
                            detail="Colour of the highlight region when the message has focus.",
                        ),
                        OptionSpec(
                            name="-highlightthickness",
                            takes_value=True,
                            detail="Width of the highlight rectangle drawn around the message.",
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
