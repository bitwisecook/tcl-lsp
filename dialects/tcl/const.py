"""const -- Define a constant variable (Tcl 9 / TIP 590)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl const(1)"


@register
class ConstCommand(CommandDef):
    name = "const"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="const",
            hover=HoverSnippet(
                summary="Define a constant variable.",
                synopsis=("const varName value",),
                snippet=(
                    "Stores ``value`` in ``varName`` and marks the variable as constant: "
                    "further attempts to ``set`` or ``unset`` it raise an error. "
                    "Re-applying ``const`` to an existing constant of the same name is a "
                    "silent no-op; applying it to an existing non-constant variable raises."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="const varName value",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2, 2),
            ),
            assigns_variable_at=0,
            arg_roles={0: frozenset({ArgRole.VAR_WRITE})},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
