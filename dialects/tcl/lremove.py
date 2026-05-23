"""lremove -- Remove elements from a list by index (Tcl 8.7+/9.0)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page lremove.n"


@register
class LremoveCommand(CommandDef):
    name = "lremove"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lremove",
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="Remove elements from a list",
                synopsis=("lremove list ?index ...?",),
                snippet="The lremove command returns a new list formed by removing the elements at the given indices from list.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lremove list ?index ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            return_type=TclType.LIST,
        )
