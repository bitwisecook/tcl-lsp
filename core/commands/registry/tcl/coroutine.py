# Scaffolded from coroutine.n -- refine and commit
"""coroutine -- Create and produce values from coroutines."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl man page coroutine.n"


@register
class CoroutineCommand(CommandDef):
    name = "coroutine"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="coroutine",
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="Create and produce values from coroutines",
                synopsis=("coroutine name command ?arg...?",),
                snippet="The coroutine command creates a new coroutine context (with associated command) named name and executes that context by calling command, passing in the other remaining arguments without further interpretation.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="coroutine name command ?arg...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            arg_roles={0: ArgRole.NAME},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
