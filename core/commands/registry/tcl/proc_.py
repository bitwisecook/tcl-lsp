"""proc -- Create a Tcl procedure."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl proc(1)"


@register
class ProcCommand(CommandDef):
    name = "proc"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="proc",
            never_inline_body=True,
            hover=HoverSnippet(
                summary="Create a Tcl procedure.",
                synopsis=("proc name args body",),
                snippet="`args` is a formal parameter list; `body` executes in a new call frame.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="proc name args body",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(3, 3),
            ),
            arg_roles={0: ArgRole.NAME, 1: ArgRole.PARAM_LIST, 2: ArgRole.BODY},
            return_type=TclType.STRING,
            defines_procedure=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.PROC_DEFINITION,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
