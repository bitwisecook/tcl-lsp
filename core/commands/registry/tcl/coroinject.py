"""coroinject -- Inject a command into a suspended coroutine."""

from __future__ import annotations

from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register


@register
class CoroinjectCommand(CommandDef):
    name = "coroinject"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="coroinject",
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="Inject a command into a suspended coroutine.",
                synopsis=("coroinject coroName command ?arg ...?",),
                source="Tcl coroinject(1)",
            ),
            forms=(
                FormSpec(kind=FormKind.DEFAULT, synopsis="coroinject coroName command ?arg ...?"),
            ),
            validation=ValidationSpec(arity=Arity(2)),
            return_type=TclType.STRING,
        )
