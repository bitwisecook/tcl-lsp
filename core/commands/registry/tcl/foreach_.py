"""foreach -- Iterate over list elements with one or more loop variables."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl foreach(1)"


@register
class ForeachCommand(CommandDef):
    name = "foreach"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="foreach",
            is_control_flow=True,
            never_inline_body=True,
            has_loop_body=True,
            hover=HoverSnippet(
                summary="Iterate over list elements with one or more loop variables.",
                synopsis=("foreach varList list ?varList list ...? body",),
                snippet="Variables are assigned from list elements; `body` runs once per assignment group.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="foreach varList list ?varList list ...? body",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(3),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(target=SideEffectTarget.UNKNOWN, connection_side=ConnectionSide.NONE),
            ),
        )
