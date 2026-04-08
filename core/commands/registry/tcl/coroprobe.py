# Scaffolded from coroutine.n -- refine and commit
"""coroprobe -- Execute a command in the context of a suspended coroutine."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page coroutine.n"


@register
class CoroprobeCommand(CommandDef):
    name = "coroprobe"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="coroprobe",
            is_language_keyword=True,
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="Execute a command in the context of a suspended coroutine.",
                synopsis=("coroprobe coroName command ?arg ...?",),
                snippet="Executes the specified command in the variable context of the given suspended coroutine.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="coroprobe coroName command ?arg ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
