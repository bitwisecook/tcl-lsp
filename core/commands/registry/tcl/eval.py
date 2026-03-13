# Scaffolded from eval.n -- refine and commit
"""eval -- Evaluate a Tcl script."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl man page eval.n"


@register
class EvalCommand(CommandDef):
    name = "eval"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="eval",
            creates_dynamic_barrier=True,
            hover=HoverSnippet(
                summary="Evaluate a Tcl script.",
                synopsis=("eval arg ?arg ...?",),
                snippet=(
                    "Concatenates its arguments and executes the result as a "
                    "Tcl script.\n\n"
                    "**Security**: If any argument contains user-controlled "
                    "data, this enables arbitrary code injection. Prefer "
                    "`{*}$cmdList` (Tcl 8.5+) to expand pre-built command "
                    "lists safely, or use direct invocation."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="eval arg ?arg ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            taint_sink=True,
            xc_translatable=False,
            arg_roles={0: ArgRole.BODY},
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(target=SideEffectTarget.UNKNOWN, connection_side=ConnectionSide.NONE),
            ),
        )
