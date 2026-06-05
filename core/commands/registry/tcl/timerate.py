"""timerate -- Measure the rate of execution of a script."""

from __future__ import annotations

from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register


@register
class TimerateCommand(CommandDef):
    name = "timerate"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="timerate",
            dialects=None,
            hover=HoverSnippet(
                summary="Measure the rate of execution of a script.",
                synopsis=(
                    "timerate ?-direct? ?-calibrate? ?-overhead double? command ?time ?max-count??",
                ),
                source="Tcl timerate(1)",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="timerate ?-direct? ?-calibrate? ?-overhead double? command ?time ?max-count??",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
            return_type=TclType.STRING,
        )
