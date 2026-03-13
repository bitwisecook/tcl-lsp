# Scaffolded from while.n -- refine and commit
"""while -- Execute script repeatedly as long as a condition is met."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ..type_hints import ArgTypeHint
from ._base import register

_SOURCE = "Tcl man page while.n"


@register
class WhileCommand(CommandDef):
    name = "while"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="while",
            is_control_flow=True,
            has_loop_body=True,
            never_inline_body=True,
            hover=HoverSnippet(
                summary="Execute script repeatedly as long as a condition is met",
                synopsis=("while test body",),
                snippet="The while command evaluates test as an expression (in the same way that expr evaluates its argument).",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="while test body",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2, 2),
            ),
            arg_roles={0: ArgRole.EXPR, 1: ArgRole.BODY},
            return_type=TclType.STRING,
            arg_types={0: ArgTypeHint(expected=TclType.BOOLEAN, shimmers=True)},
            side_effect_hints=(
                SideEffect(target=SideEffectTarget.UNKNOWN, connection_side=ConnectionSide.NONE),
            ),
        )
