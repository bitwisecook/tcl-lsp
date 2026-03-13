"""exec -- Invoke subprocesses."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    OptionSpec,
    OptionTerminatorSpec,
    ValidationSpec,
)
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import register


@register
class ExecCommand(CommandDef):
    name = "exec"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="exec",
            dialects=DIALECTS_EXCEPT_IRULES,
            option_terminator_profiles=(OptionTerminatorSpec(scan_start=0),),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="exec ?switches? arg ?arg ...?",
                    options=(
                        OptionSpec(name="-ignorestderr"),
                        OptionSpec(name="-keepnewline"),
                        OptionSpec(name="--"),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            taint_sink=True,
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
