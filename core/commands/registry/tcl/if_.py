"""if -- Conditional execution with optional elseif/else branches."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ..type_hints import ArgTypeHint
from ._base import register

_SOURCE = "Tcl if(1)"


@register
class IfCommand(CommandDef):
    name = "if"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="if",
            is_control_flow=True,
            never_inline_body=True,
            hover=HoverSnippet(
                summary="Conditional execution with optional elseif/else branches.",
                synopsis=("if expr1 ?then? body1 ?elseif expr2 ?then? body2 ...? ?else? ?bodyN?",),
                snippet="Expressions are evaluated left-to-right until a true branch is selected.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="if expr1 ?then? body1 ?elseif expr2 ?then? body2 ...? ?else? ?bodyN?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            arg_roles={0: ArgRole.EXPR, 1: ArgRole.BODY, 3: ArgRole.BODY, 5: ArgRole.BODY},
            return_type=TclType.STRING,
            arg_types={0: ArgTypeHint(expected=TclType.BOOLEAN, shimmers=True)},
            side_effect_hints=(
                SideEffect(target=SideEffectTarget.UNKNOWN, connection_side=ConnectionSide.NONE),
            ),
        )
