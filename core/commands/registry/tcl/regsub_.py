"""regsub -- Perform substitutions based on regular expression matching."""

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
    PatternType,
    ValidationSpec,
)
from ..signatures import ArgRole, Arity
from ._base import register


def _regsub_arg_role_resolver(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """Dynamically assign VAR_WRITE to the regsub result variable.

    ``regsub ?switches? exp string subSpec ?varName?``

    After skipping options, arg 0 = pattern, arg 1 = string,
    arg 2 = subSpec, arg 3 = varName (optional, written to).
    """
    from ..runtime import options_with_value, skip_options

    first_positional = skip_options(args, options_with_value("regsub"))
    var_idx = first_positional + 3
    if var_idx < len(args):
        return {var_idx: frozenset({ArgRole.VAR_WRITE})}
    return {}


@register
class RegsubCommand(CommandDef):
    name = "regsub"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="regsub",
            hover=HoverSnippet(
                summary="Perform substitutions based on regular expression matching.",
                synopsis=("regsub ?switches? exp string subSpec ?varName?",),
                snippet=(
                    "Matches *exp* against *string* and replaces the matched "
                    "portion with *subSpec*. With `-all`, replaces all "
                    "occurrences.\n\n"
                    "**Security**: Use `--` before the pattern when it comes "
                    "from a variable to prevent option injection. The "
                    "*subSpec* supports `\\0`..`\\9` backreferences and `&` "
                    "for the full match."
                ),
                source="Tcl regsub(1)",
                return_value=(
                    "The substituted string (Tcl 8.5+), or the count of "
                    "replacements when *varName* is given."
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="regsub ?switches? exp string subSpec ?varName?",
                    options=(
                        OptionSpec(name="-nocase"),
                        OptionSpec(name="-expanded"),
                        OptionSpec(name="-line"),
                        OptionSpec(name="-linestop"),
                        OptionSpec(name="-lineanchor"),
                        OptionSpec(name="-all"),
                        OptionSpec(name="-start", takes_value=True, value_hint="index"),
                        OptionSpec(name="--"),
                    ),
                ),
            ),
            arg_role_resolver=_regsub_arg_role_resolver,
            validation=ValidationSpec(
                arity=Arity(3, 4),
            ),
            pattern_type=PatternType.REGEX,
            return_type=TclType.INT,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.VARIABLE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
