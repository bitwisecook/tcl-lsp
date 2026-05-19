"""foreach -- Iterate over list elements with one or more loop variables."""

from __future__ import annotations

from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register
from .shimmer_resolvers import resolve_foreach

_SOURCE = "Tcl foreach(1)"


def _foreach_arg_roles(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """Last argument is the body script."""
    if len(args) >= 3:
        return {len(args) - 1: frozenset({ArgRole.BODY})}
    return {}


@register
class ForeachCommand(CommandDef):
    name = "foreach"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="foreach",
            is_control_flow=True,
            is_language_keyword=True,
            never_inline_body=True,
            has_loop_body=True,
            loop_list_header=True,
            hover=HoverSnippet(
                summary="Iterate over list elements with one or more loop variables.",
                synopsis=("foreach varList list ?varList list ...? body",),
                snippet="Variables are assigned from list elements; `body` runs once per assignment group.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="foreach varList list ?varList list ...? body",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(3),
            ),
            wasm_emits_nothing=True,
            arg_role_resolver=_foreach_arg_roles,
            return_type=TclType.STRING,
            arg_type_resolver=resolve_foreach,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
