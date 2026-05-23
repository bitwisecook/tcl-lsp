"""foreachLine -- Iterate over the lines of a file (Tcl 9.0+, TIP 670)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page library.n"


@register
class ForeachLineCommand(CommandDef):
    name = "foreachLine"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="foreachLine",
            dialects=frozenset({"tcl9.0"}),
            is_control_flow=True,
            has_loop_body=True,
            never_inline_body=True,
            hover=HoverSnippet(
                summary="Iterate over the lines of a text file, one line at a time.",
                synopsis=("foreachLine varName filename body",),
                snippet=(
                    "Reads the text file *filename* one line at a time, "
                    "assigns each line to *varName*, and evaluates *body*. "
                    "`break`, `continue`, `return`, and `error` behave as in "
                    "`foreach`. The file is closed before the procedure "
                    "returns. Introduced by TIP 670 in Tcl 9.0."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="foreachLine varName filename body",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(3, 3),
            ),
            arg_roles={
                0: frozenset({ArgRole.VAR_WRITE}),
                2: frozenset({ArgRole.BODY}),
            },
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
