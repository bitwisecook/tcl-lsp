"""ttk::scale -- Themed scale (slider) widget."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page ttk_scale.n"

_av = make_av(_SOURCE)

_ORIENT_VALUES = (
    _av("horizontal", "Horizontal slider orientation."),
    _av("vertical", "Vertical slider orientation."),
)


@register
class TtkScaleCommand(CommandDef):
    name = "ttk::scale"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ttk::scale",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a themed scale (slider) widget.",
                synopsis=("ttk::scale pathName ?options?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ttk::scale pathName ?options?",
                    options=(
                        OptionSpec(
                            name="-from",
                            takes_value=True,
                            value_hint="value",
                            detail="Starting value of the scale range.",
                        ),
                        OptionSpec(
                            name="-to",
                            takes_value=True,
                            value_hint="value",
                            detail="Ending value of the scale range.",
                        ),
                        OptionSpec(
                            name="-value",
                            takes_value=True,
                            value_hint="value",
                            detail="Current value of the scale.",
                        ),
                        OptionSpec(
                            name="-variable",
                            takes_value=True,
                            value_hint="varName",
                            detail="Variable linked to the scale value.",
                        ),
                        OptionSpec(
                            name="-orient",
                            takes_value=True,
                            value_hint="orientation",
                            detail="Orientation of the scale (horizontal or vertical).",
                        ),
                        OptionSpec(
                            name="-length",
                            takes_value=True,
                            value_hint="length",
                            detail="Length of the long axis of the scale widget.",
                        ),
                        OptionSpec(
                            name="-command",
                            takes_value=True,
                            value_hint="script",
                            detail="Script to evaluate when the scale value changes.",
                        ),
                        OptionSpec(
                            name="-state",
                            takes_value=True,
                            value_hint="stateSpec",
                            detail="Widget state (normal or disabled).",
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
