"""readFile -- Read the contents of a file and return them."""

from __future__ import annotations

from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register


@register
class ReadfileCommand(CommandDef):
    name = "readFile"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="readFile",
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="Read the contents of a file and return them.",
                synopsis=("readFile filename ?text|binary?",),
                source="Tcl man page library.n",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="readFile filename ?text|binary?"),),
            validation=ValidationSpec(arity=Arity(1, 2)),
            return_type=TclType.STRING,
        )
