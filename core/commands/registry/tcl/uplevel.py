# Scaffolded from uplevel.n -- refine and commit
"""uplevel -- Execute a script in a different stack frame."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ..taint_hints import TaintColour
from ._base import register

_SOURCE = "Tcl man page uplevel.n"


def _uplevel_arg_roles(args: list[str]) -> dict[int, ArgRole]:
    """Resolve arg roles for ``uplevel ?level? arg ?arg ...?``

    If the first argument looks like a stack level (integer or ``#N``),
    all remaining arguments are BODY.  Otherwise all arguments are BODY.
    """
    if not args:
        return {}
    first = args[0]
    if first.isdigit() or (first.startswith("#") and len(first) > 1 and first[1:].isdigit()):
        return {i: ArgRole.BODY for i in range(1, len(args))}
    return {i: ArgRole.BODY for i in range(len(args))}


@register
class UplevelCommand(CommandDef):
    name = "uplevel"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="uplevel",
            creates_dynamic_barrier=True,
            unsafe=True,
            hover=HoverSnippet(
                summary="Execute a script in a different stack frame",
                synopsis=("uplevel ?level? arg ?arg ...?",),
                snippet="All of the arg arguments are concatenated as if they had been passed to concat; the result is then evaluated in the variable context indicated by level.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="uplevel ?level? arg ?arg ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            arg_role_resolver=_uplevel_arg_roles,
            taint_sink=True,
            taint_sink_safe_colour=TaintColour.LIST_CANONICAL,
            xc_translatable=False,
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(target=SideEffectTarget.UNKNOWN, connection_side=ConnectionSide.NONE),
            ),
        )
