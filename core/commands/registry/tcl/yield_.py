# Scaffolded from coroutine.n -- refine and commit
"""yield -- Create and produce values from coroutines."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page coroutine.n"


@register
class YieldCommand(CommandDef):
    name = "yield"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="yield",
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="Create and produce values from coroutines",
                synopsis=("yield ?value?",),
                snippet="The coroutine command creates a new coroutine context (with associated command) named name and executes that context by calling command, passing in the other remaining arguments without further interpretation.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="yield ?value?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0, 1),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE, connection_side=ConnectionSide.NONE
                ),
            ),
        )
