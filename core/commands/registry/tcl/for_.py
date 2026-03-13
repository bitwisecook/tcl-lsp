"""for -- C-style loop with init, test, and next scripts."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ..type_hints import ArgTypeHint
from ._base import register

_SOURCE = "Tcl for(1)"


@register
class ForCommand(CommandDef):
    name = "for"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="for",
            is_control_flow=True,
            never_inline_body=True,
            has_loop_body=True,
            hover=HoverSnippet(
                summary="C-style loop with init, test, and next scripts.",
                synopsis=("for start test next body",),
                snippet="`start` runs once; loop continues while `test` is true; `next` runs after each body pass.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="for start test next body",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(4, 4),
            ),
            arg_roles={0: ArgRole.BODY, 1: ArgRole.EXPR, 2: ArgRole.BODY, 3: ArgRole.BODY},
            return_type=TclType.STRING,
            arg_types={1: ArgTypeHint(expected=TclType.BOOLEAN, shimmers=True)},
            side_effect_hints=(
                SideEffect(target=SideEffectTarget.UNKNOWN, connection_side=ConnectionSide.NONE),
            ),
        )
