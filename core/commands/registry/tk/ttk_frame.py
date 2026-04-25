"""ttk::frame -- Themed frame container widget."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page ttk_frame.n"


@register
class TtkFrameCommand(CommandDef):
    name = "ttk::frame"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ttk::frame",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a themed frame container widget.",
                synopsis=("ttk::frame pathName ?options?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ttk::frame pathName ?options?",
                    options=(
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            value_hint="width",
                            detail="Desired width of the frame.",
                        ),
                        OptionSpec(
                            name="-height",
                            takes_value=True,
                            value_hint="height",
                            detail="Desired height of the frame.",
                        ),
                        OptionSpec(
                            name="-relief",
                            takes_value=True,
                            value_hint="relief",
                            detail="Border relief style for the frame.",
                        ),
                        OptionSpec(
                            name="-borderwidth",
                            takes_value=True,
                            value_hint="width",
                            detail="Width of the frame border.",
                        ),
                        OptionSpec(
                            name="-padding",
                            takes_value=True,
                            value_hint="padSpec",
                            detail="Internal padding around the frame content.",
                        ),
                        OptionSpec(
                            name="-style",
                            takes_value=True,
                            value_hint="style",
                            detail="Style to use for the widget.",
                        ),
                        OptionSpec(
                            name="-class",
                            takes_value=True,
                            value_hint="className",
                            detail="Widget class name for option-database lookups.",
                        ),
                        OptionSpec(
                            name="-cursor",
                            takes_value=True,
                            value_hint="cursor",
                            detail="Cursor to display when the pointer is over the widget.",
                        ),
                        OptionSpec(
                            name="-takefocus",
                            takes_value=True,
                            value_hint="focusSpec",
                            detail="Whether the widget accepts focus during keyboard traversal.",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
