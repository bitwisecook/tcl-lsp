"""regexp -- Match a regular expression against a string."""

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


def _regexp_arg_role_resolver(args: list[str]) -> dict[int, ArgRole]:
    """Dynamically assign VAR_WRITE to regexp capture variables.

    ``regexp ?switches? exp string ?matchVar? ?subMatchVar ...?``

    After skipping options, arg 0 = pattern, arg 1 = string, and
    args 2+ are capture variable names.  We need to map the raw
    argument indices (including options) to the VAR_WRITE role.
    """
    from ..runtime import options_with_value, skip_options

    first_positional = skip_options(args, options_with_value("regexp"))
    # Capture variables start 2 past the first positional arg.
    result: dict[int, ArgRole] = {}
    capture_start = first_positional + 2
    for i in range(capture_start, len(args)):
        result[i] = ArgRole.VAR_WRITE
    return result


@register
class RegexpCommand(CommandDef):
    name = "regexp"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="regexp",
            hover=HoverSnippet(
                summary="Match a regular expression against a string.",
                synopsis=("regexp ?switches? exp string ?matchVar? ?subMatchVar ...?",),
                snippet=(
                    "Returns 1 if *exp* matches part of *string*, 0 otherwise. "
                    "Matching substrings are stored in *matchVar* and "
                    "*subMatchVar*.\n\n"
                    "**Security**: Use `--` before the pattern when it comes "
                    "from a variable to prevent option injection. Avoid nested "
                    "quantifiers like `(a+)+` which can cause catastrophic "
                    "backtracking (ReDoS) on crafted input."
                ),
                source="Tcl regexp(1)",
                return_value="1 if the pattern matches, 0 otherwise.",
            ),
            warn_without_terminator=True,
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="regexp ?switches? exp string ?matchVar? ?subMatchVar ...?",
                    options=(
                        OptionSpec(name="-nocase"),
                        OptionSpec(name="-expanded"),
                        OptionSpec(name="-line"),
                        OptionSpec(name="-linestop"),
                        OptionSpec(name="-lineanchor"),
                        OptionSpec(name="-all"),
                        OptionSpec(name="-inline"),
                        OptionSpec(name="-indices"),
                        OptionSpec(name="-start", takes_value=True, value_hint="index"),
                        OptionSpec(name="-about"),
                        OptionSpec(name="--"),
                    ),
                ),
            ),
            arg_role_resolver=_regexp_arg_role_resolver,
            validation=ValidationSpec(
                arity=Arity(1),
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
