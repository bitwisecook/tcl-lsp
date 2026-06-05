"""foreachLine -- Iterate over the lines of a text file, one line at a time."""

from __future__ import annotations

from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register


@register
class ForeachlineCommand(CommandDef):
    name = "foreachLine"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="foreachLine",
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="Iterate over the lines of a text file, one line at a time.",
                synopsis=("foreachLine varName filename body",),
                source="Tcl man page library.n",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="foreachLine varName filename body"),),
            validation=ValidationSpec(arity=Arity(3, 3)),
            return_type=TclType.STRING,
        )
