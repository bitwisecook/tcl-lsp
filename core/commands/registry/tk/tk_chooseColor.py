"""tk_chooseColor -- Display a colour chooser dialogue."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page tk_chooseColor.n"


@register
class TkChooseColorCommand(CommandDef):
    name = "tk_chooseColor"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tk_chooseColor",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Pop up a dialogue for the user to select a colour.",
                synopsis=("tk_chooseColor ?option value ...?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="tk_chooseColor ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-initialcolor",
                            takes_value=True,
                            value_hint="colour",
                            detail="Initial colour to display in the chooser.",
                        ),
                        OptionSpec(
                            name="-parent",
                            takes_value=True,
                            value_hint="window",
                            detail="Parent window for the dialogue.",
                        ),
                        OptionSpec(
                            name="-title",
                            takes_value=True,
                            value_hint="titleString",
                            detail="Title string for the dialogue window.",
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
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
