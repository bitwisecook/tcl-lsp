# Scaffolded from lset.n -- refine and commit
"""lset -- Change an element in a list."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.registry.type_hints import ArgTypeHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageType
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page lset.n"


@register
class LsetCommand(CommandDef):
    name = "lset"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lset",
            hover=HoverSnippet(
                summary="Change an element in a list",
                synopsis=("lset varName ?index ...? newValue",),
                snippet="The lset command accepts a parameter, varName, which it interprets as the name of a variable containing a Tcl list.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lset varName ?index ...? newValue",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            assigns_variable_at=0,
            arg_roles={0: frozenset({ArgRole.VAR_WRITE})},
            return_type=TclType.LIST,
            inferred_storage_type=StorageType.LIST,
            arg_types={0: ArgTypeHint(expected=TclType.LIST, shimmers=True)},
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
