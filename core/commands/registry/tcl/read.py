# Scaffolded from read.n -- refine and commit
"""read -- Read from a channel."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import register

_SOURCE = "Tcl man page read.n"


@register
class ReadCommand(CommandDef):
    name = "read"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="read",
            hover=HoverSnippet(
                summary="Read from a channel",
                synopsis=(
                    "read ?-nonewline? channel",
                    "read channel numChars",
                ),
                snippet="The read command has been superceded by the chan read command which supports the same syntax and options.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="read ?-nonewline? channel",
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

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
