# Scaffolded from list.n -- refine and commit
"""list -- Create a list."""

from __future__ import annotations

from compiler.types import TclType

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register
from .const_fold import fold_list

_SOURCE = "Tcl man page list.n"


@register
class ListCommand(CommandDef):
    name = "list"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="list",
            hover=HoverSnippet(
                summary="Create a list",
                synopsis=("list ?arg arg ...?",),
                snippet="This command returns a list comprised of all the args, or an empty string if no args are specified.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="list ?arg arg ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            pure=True,
            const_fold=fold_list,
            produces_canonical_list=True,
            return_type=TclType.LIST,
            side_effect_hints=(),
        )
