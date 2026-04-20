"""bell -- Ring the display's bell."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page bell.n"


@register
class BellCommand(CommandDef):
    name = "bell"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="bell",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Ring the display's bell.",
                synopsis=("bell ?-displayof window? ?-nice?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="bell ?-displayof window? ?-nice?",
                    options=(
                        OptionSpec(
                            name="-displayof",
                            takes_value=True,
                            value_hint="window",
                            detail="Specifies the display on which to ring the bell.",
                        ),
                        OptionSpec(
                            name="-nice",
                            takes_value=False,
                            detail="Do not reset the screen saver when ringing the bell.",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
