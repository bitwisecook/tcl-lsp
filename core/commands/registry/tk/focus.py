"""focus -- Manage the input focus."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page focus.n"
_av = make_av(_SOURCE)


@register
class FocusCommand(CommandDef):
    name = "focus"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="focus",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Manage the input focus.",
                synopsis=(
                    "focus",
                    "focus window",
                    "focus -displayof window",
                    "focus -force window",
                    "focus -lastfor window",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="focus ?option? ?window?",
                    options=(
                        OptionSpec(
                            name="-displayof",
                            takes_value=True,
                            value_hint="window",
                            detail="Return the focus window on the display of the given window.",
                        ),
                        OptionSpec(
                            name="-force",
                            takes_value=True,
                            value_hint="window",
                            detail="Set the focus to the window even if the application does not currently have focus.",
                        ),
                        OptionSpec(
                            name="-lastfor",
                            takes_value=True,
                            value_hint="window",
                            detail="Return the name of the most recent window to have the input focus among the window's top-level.",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0, 2),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
