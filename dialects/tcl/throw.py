# Scaffolded from throw.n -- refine and commit
"""throw -- Generate a machine-readable error."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page throw.n"


@register
class ThrowCommand(CommandDef):
    name = "throw"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="throw",
            is_language_keyword=True,
            terminates_block=True,
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="Generate a machine-readable error",
                synopsis=("throw type message",),
                snippet="This command causes the current evaluation to be unwound with an error.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="throw type message",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2, 2),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
