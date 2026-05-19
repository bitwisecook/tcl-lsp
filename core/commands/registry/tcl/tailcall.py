# Scaffolded from tailcall.n -- refine and commit
"""tailcall -- Replace the current procedure with another command."""

from __future__ import annotations

from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page tailcall.n"


@register
class TailcallCommand(CommandDef):
    name = "tailcall"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tailcall",
            is_language_keyword=True,
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="Replace the current procedure with another command",
                synopsis=("tailcall command ?arg ...?",),
                snippet="The tailcall command replaces the currently executing procedure, lambda application, or method with another command.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="tailcall command ?arg ...?",
                ),
            ),
            validation=ValidationSpec(
                # C Tcl 9.0 ``TclNRTailcallObjCmd``: ``if (objc < 1)``
                # is unreachable, so any arg count is accepted.
                # Invocation without args clears a previously
                # scheduled tailcall; with args it replaces it.  So
                # the real arity is 0..∞.
                arity=Arity(0),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE, connection_side=ConnectionSide.NONE
                ),
            ),
        )
