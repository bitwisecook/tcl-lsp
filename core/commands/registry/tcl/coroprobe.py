"""coroprobe -- Evaluate a command in a suspended coroutine."""

from __future__ import annotations

from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register


@register
class CoroprobeCommand(CommandDef):
    name = "coroprobe"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="coroprobe",
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="Evaluate a command in a suspended coroutine.",
                synopsis=("coroprobe coroName command ?arg ...?",),
                source="Tcl coroprobe(1)",
            ),
            forms=(
                FormSpec(kind=FormKind.DEFAULT, synopsis="coroprobe coroName command ?arg ...?"),
            ),
            validation=ValidationSpec(arity=Arity(2)),
            return_type=TclType.STRING,
        )
