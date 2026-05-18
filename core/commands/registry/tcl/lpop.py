"""lpop -- Get and remove an element from a list variable (Tcl 9.0+, TIP 323)."""

from __future__ import annotations

from ....compiler.side_effects import StorageType
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ..type_hints import ArgTypeHint
from ._base import register

_SOURCE = "Tcl 9 man page lpop.n"


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
                snippet=(
                    "Removes the element at the given indices from the list "
                    "stored in `varName` (defaulting to `end`) and returns "
                    "it. Nested indices descend into sublists, like `lindex` "
                    "/ `lset`. Introduced in Tcl 9.0."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lpop varName ?index ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            pure=False,
            return_type=TclType.STRING,
            inferred_storage_type=StorageType.LIST,
            assigns_variable_at=0,
            arg_roles={0: frozenset({ArgRole.VAR_WRITE})},
            arg_types={0: ArgTypeHint(expected=TclType.LIST, shimmers=True)},
            side_effect_hints=(),
        )
