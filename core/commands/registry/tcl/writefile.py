"""writeFile -- Write contents to a file."""

from __future__ import annotations

from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register


@register
class WritefileCommand(CommandDef):
    name = "writeFile"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="writeFile",
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="Write contents to a file.",
                synopsis=("writeFile filename ?text|binary? contents",),
                source="Tcl man page library.n",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT, synopsis="writeFile filename ?text|binary? contents"
                ),
            ),
            validation=ValidationSpec(arity=Arity(2, 3)),
            return_type=TclType.STRING,
        )
