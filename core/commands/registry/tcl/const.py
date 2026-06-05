"""const -- Define a constant variable."""

from __future__ import annotations

from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register


@register
class ConstCommand(CommandDef):
    name = "const"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="const",
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="Define a constant variable.",
                synopsis=("const varName value",),
                source="Tcl const(1)",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="const varName value"),),
            validation=ValidationSpec(arity=Arity(2, 2)),
            return_type=TclType.STRING,
        )
