"""lpop -- Get and remove an element in a list variable."""

from __future__ import annotations

from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register


@register
class LpopCommand(CommandDef):
    name = "lpop"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lpop",
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="Get and remove an element in a list variable.",
                synopsis=("lpop varName ?index ...?",),
                source="Tcl 9 man page lpop.n",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="lpop varName ?index ...?"),),
            validation=ValidationSpec(arity=Arity(1)),
            return_type=TclType.STRING,
        )
