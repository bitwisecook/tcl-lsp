# Scaffolded from gets.n -- refine and commit
"""gets -- Read a line from a channel."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import register

_SOURCE = "Tcl man page gets.n"


@register
class GetsCommand(CommandDef):
    name = "gets"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="gets",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Read a line from a channel",
                synopsis=("gets channel ?varName?",),
                snippet="The gets command has been superceded by the chan gets command which supports the same syntax and options.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="gets channel ?varName?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 2),
            ),
            arg_roles={1: ArgRole.VAR_NAME},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
