"""source -- Evaluate a file or resource as a Tcl script."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl source(1)"


@register
class SourceCommand(CommandDef):
    name = "source"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="source",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Evaluate a file or resource as a Tcl script.",
                synopsis=("source ?-encoding name? fileName",),
                snippet="",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="source ?-encoding name? fileName",
                    options=(
                        OptionSpec(
                            name="-encoding",
                            takes_value=True,
                            value_hint="encoding",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 2),
            ),
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
