"""expr -- Evaluate a Tcl expression."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ..type_hints import ArgTypeHint
from ._base import register

_SOURCE = "Tcl expr(1)"


@register
class ExprCommand(CommandDef):
    name = "expr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="expr",
            needs_start_cmd=True,
            hover=HoverSnippet(
                summary="Evaluate a Tcl expression.",
                synopsis=("expr arg ?arg ...?",),
                snippet=(
                    "**Always brace expressions**: `expr {$a + $b}`.\n\n"
                    "Without braces, `expr $x + 1` undergoes double "
                    "substitution: the Tcl parser expands `$x` first, then "
                    "`expr` evaluates the result. If `$x` contains "
                    "`[dangerous_command]`, it executes. Bracing also enables "
                    "bytecode compilation for better performance."
                ),
                source=_SOURCE,
                return_value="The result of evaluating the expression.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="expr arg ?arg ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            taint_sink=True,
            arg_roles={0: ArgRole.EXPR},
            return_type=TclType.NUMERIC,
            arg_types={0: ArgTypeHint(expected=TclType.NUMERIC, shimmers=True)},
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
