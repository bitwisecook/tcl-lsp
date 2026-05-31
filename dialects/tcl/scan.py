# Scaffolded from scan.n -- refine and commit
"""scan -- Parse string using conversion specifiers in the style of sscanf."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page scan.n"


def _scan_arg_roles(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """D4-F2 closure: scan accepts variable-name args from index 2
    onward to the end of the call (``scan STRING FORMAT ?var var ...?``).
    Rather than hard-coding a finite slot count, return VAR_WRITE for
    every trailing arg dynamically, so calls with 20 / 50 / 100 vars
    don't false-fire W210 on the unmodelled tail."""
    return {i: frozenset({ArgRole.VAR_WRITE}) for i in range(2, len(args))}


@register
class ScanCommand(CommandDef):
    name = "scan"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="scan",
            byte_compiled=True,
            hover=HoverSnippet(
                summary="Parse string using conversion specifiers in the style of sscanf",
                synopsis=("scan string format ?varName varName ...?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="scan string format ?varName varName ...?",
                ),
            ),
            arg_role_resolver=_scan_arg_roles,
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            return_type=TclType.INT,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
