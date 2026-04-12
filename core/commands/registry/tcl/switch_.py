"""switch -- Pattern-based branching on a subject string."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl switch(1)"

_SWITCH_VALUE_OPTIONS = frozenset({"-matchvar", "-indexvar"})


def _switch_arg_roles(args: list[str]) -> dict[int, ArgRole]:
    """Resolve BODY roles for switch command."""
    # Skip option flags.
    i = 0
    while i < len(args):
        a = args[i]
        if a == "--":
            i += 1
            break
        if not a.startswith("-"):
            break
        if a in _SWITCH_VALUE_OPTIONS:
            i += 2
        else:
            i += 1
    # Skip switch value.
    if i < len(args):
        i += 1
    if i >= len(args):
        return {}
    roles: dict[int, ArgRole] = {}
    # Braced list form: single trailing argument.
    if i == len(args) - 1:
        roles[i] = ArgRole.BODY
        return roles
    # List form: pattern body pairs.
    while i + 1 < len(args):
        if args[i + 1] != "-":
            roles[i + 1] = ArgRole.BODY
        i += 2
    return roles


@register
class SwitchCommand(CommandDef):
    name = "switch"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="switch",
            is_control_flow=True,
            is_language_keyword=True,
            never_inline_body=True,
            hover=HoverSnippet(
                summary="Pattern-based branching on a subject string.",
                synopsis=(
                    "switch ?options? string pattern body ?pattern body ...?",
                    "switch ?options? string {pattern body ?pattern body ...?}",
                ),
                snippet="Use `-exact`, `-glob`, or `-regexp` to select matching mode.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="switch ?options? string pattern body ?pattern body ...?",
                    options=(
                        OptionSpec(name="-exact", detail="Exact string compare mode."),
                        OptionSpec(name="-glob", detail="Glob pattern mode."),
                        OptionSpec(name="-regexp", detail="Regular expression mode."),
                        OptionSpec(name="-nocase", detail="Case-insensitive matching."),
                        OptionSpec(
                            name="-matchvar",
                            detail="Store match in variable (regexp mode).",
                            takes_value=True,
                            value_hint="varName",
                        ),
                        OptionSpec(
                            name="-indexvar",
                            detail="Store match indices in variable (regexp mode).",
                            takes_value=True,
                            value_hint="varName",
                        ),
                        OptionSpec(name="--", detail="End of options."),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            arg_role_resolver=_switch_arg_roles,
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
            has_switch_body=True,
        )
