"""tk_popup -- Post a pop-up menu."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page tk_popup.n"


@register
class TkPopupCommand(CommandDef):
    name = "tk_popup"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tk_popup",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Post a pop-up menu at the given screen coordinates.",
                synopsis=("tk_popup menu x y ?entry?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="tk_popup menu x y ?entry?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(3, 4),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
