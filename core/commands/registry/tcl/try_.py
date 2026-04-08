# Scaffolded from try.n -- refine and commit
"""try -- Trap and process errors and exceptions."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl man page try.n"


def _try_arg_roles(args: list[str]) -> dict[int, ArgRole]:
    """Resolve BODY roles for try/on/trap/finally."""
    roles: dict[int, ArgRole] = {}
    if args:
        roles[0] = ArgRole.BODY
    i = 1
    while i < len(args):
        kw = args[i]
        if kw == "finally" and i + 1 < len(args):
            roles[i + 1] = ArgRole.BODY
            i += 2
        elif kw in ("on", "trap") and i + 3 < len(args):
            roles[i + 2] = ArgRole.VAR_NAME
            roles[i + 3] = ArgRole.BODY
            i += 4
        else:
            i += 1
    return roles


@register
class TryCommand(CommandDef):
    name = "try"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="try",
            is_control_flow=True,
            is_language_keyword=True,
            never_inline_body=True,
            dialects=frozenset({"tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="Trap and process errors and exceptions",
                synopsis=("try body ?handler...? ?finally script?",),
                snippet="This command executes the script body and, depending on what the outcome of that script is (normal exit, error, or some other exceptional result), runs a handler script to deal with the case.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="try body ?handler...? ?finally script?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            arg_role_resolver=_try_arg_roles,
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
