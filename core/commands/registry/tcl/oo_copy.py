# Scaffolded from copy.n -- refine and commit
"""oo::copy -- create copies of objects and classes."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page copy.n"


@register
class OoCopyCommand(CommandDef):
    name = "oo::copy"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="oo::copy",
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="create copies of objects and classes",
                synopsis=("oo::copy sourceObject ?targetObject? ?targetNamespace?",),
                snippet="The oo::copy command creates a copy of an object or class.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="oo::copy sourceObject ?targetObject? ?targetNamespace?",
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
