# Scaffolded from try.n -- refine and commit
"""try -- Trap and process errors and exceptions."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl man page try.n"


@register
class TryCommand(CommandDef):
    name = "try"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="try",
            is_control_flow=True,
            never_inline_body=True,
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="Trap and process errors and exceptions",
                synopsis=("try body ?handler...? ?finally script?",),
                snippet="This command executes the script body and, depending on what the outcome of that script is (normal exit, error, or some other exceptional result), runs a handler script to deal with the case.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="try body ?handler...? ?finally script?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            arg_roles={0: ArgRole.BODY},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(target=SideEffectTarget.UNKNOWN, connection_side=ConnectionSide.NONE),
            ),
        )
