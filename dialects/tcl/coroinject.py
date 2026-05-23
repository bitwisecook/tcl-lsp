# Based on the Tcl coroutine.n man page.
"""coroinject -- Inject a command into a suspended coroutine."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import register

_SOURCE = "Tcl man page coroutine.n"


@register
class CoroinjectCommand(CommandDef):
    name = "coroinject"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="coroinject",
            is_language_keyword=True,
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="Inject a command into a suspended coroutine.",
                synopsis=("coroinject coroName command ?arg ...?",),
                snippet="Arranges for the specified command to be executed inside the given coroutine the next time it resumes.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="coroinject coroName command ?arg ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
