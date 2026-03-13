"""ttk::separator -- Themed separator widget."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page ttk_separator.n"

_av = make_av(_SOURCE)

_ORIENT_VALUES = (
    _av("horizontal", "Horizontal separator line."),
    _av("vertical", "Vertical separator line."),
)


@register
class TtkSeparatorCommand(CommandDef):
    name = "ttk::separator"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ttk::separator",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a themed separator widget.",
                synopsis=("ttk::separator pathName ?options?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ttk::separator pathName ?options?",
                    options=(
                        OptionSpec(
                            name="-orient",
                            takes_value=True,
                            value_hint="orientation",
                            detail="Orientation of the separator (horizontal or vertical).",
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
