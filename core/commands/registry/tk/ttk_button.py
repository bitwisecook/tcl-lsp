"""ttk::button -- Themed button widget."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page ttk_button.n"

_av = make_av(_SOURCE)

_STATE_VALUES = (
    _av("normal", "The default state; the button is active and responsive."),
    _av("disabled", "The button is greyed out and unresponsive."),
)

_DEFAULT_VALUES = (
    _av("normal", "Normal button appearance."),
    _av("active", "The button is drawn as the default button."),
    _av("disabled", "The button is drawn as the default button but is disabled."),
)


@register
class TtkButtonCommand(CommandDef):
    name = "ttk::button"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ttk::button",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a themed button widget.",
                synopsis=("ttk::button pathName ?options?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ttk::button pathName ?options?",
                    options=(
                        OptionSpec(
                            name="-text",
                            takes_value=True,
                            value_hint="string",
                            detail="Text to display in the button.",
                        ),
                        OptionSpec(
                            name="-textvariable",
                            takes_value=True,
                            value_hint="varName",
                            detail="Variable whose value is used as the button text.",
                        ),
                        OptionSpec(
                            name="-command",
                            takes_value=True,
                            value_hint="script",
                            detail="Script to evaluate when the button is invoked.",
                        ),
                        OptionSpec(
                            name="-image",
                            takes_value=True,
                            value_hint="imageName",
                            detail="Image to display in the button.",
                        ),
                        OptionSpec(
                            name="-compound",
                            takes_value=True,
                            value_hint="compoundType",
                            detail="How to display image relative to text.",
                        ),
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            value_hint="width",
                            detail="Desired width of the button.",
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
                        OptionSpec(
                            name="-padding",
                            takes_value=True,
                            value_hint="padSpec",
                            detail="Internal padding around the widget content.",
                        ),
                        OptionSpec(
                            name="-underline",
                            takes_value=True,
                            value_hint="index",
                            detail="Index of the character to underline for mnemonic activation.",
                        ),
                        OptionSpec(
                            name="-default",
                            takes_value=True,
                            value_hint="defaultState",
                            detail="Default button state (normal, active, or disabled).",
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
