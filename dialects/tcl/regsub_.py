"""regsub -- Perform substitutions based on regular expression matching."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    PatternType,
    ValidationSpec,
)
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register


def _regsub_arg_role_resolver(args: list[str]) -> dict[int, frozenset[ArgRole]]:
    """Dynamically assign roles to regsub's positional arguments.

    ``regsub ?switches? exp string subSpec ?varName?``

    After skipping options, arg 0 = pattern, arg 1 = string,
    arg 2 = subSpec, arg 3 = varName (optional, written to).

    When ``-command`` is present (TIP 463, Tcl 9.0+), *subSpec* is a
    Tcl command prefix invoked with the matched substrings appended as
    arguments; its return value becomes the replacement.  We tag it
    with :class:`ArgRole.NAME` in that case so the analyser knows it
    refers to a callable identifier rather than a substitution
    template string.
    """
    from compiler.registry.runtime import options_with_value, skip_options

    options_set = options_with_value("regsub")
    first_positional = skip_options(args, options_set)
    roles: dict[int, frozenset[ArgRole]] = {}

    # Detect ``-command`` among the leading switches.
    has_command_option = any(arg == "-command" for arg in args[:first_positional])
    sub_spec_idx = first_positional + 2
    if has_command_option and sub_spec_idx < len(args):
        roles[sub_spec_idx] = frozenset({ArgRole.NAME})

    var_idx = first_positional + 3
    if var_idx < len(args):
        roles[var_idx] = frozenset({ArgRole.VAR_WRITE})
    return roles


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
                        OptionSpec(
                            name="-command",
                            detail="Treat subSpec as a command prefix called with each match.",
                            dialects=frozenset({"tcl9.0"}),
                            hover=HoverSnippet(
                                summary=(
                                    "Treat *subSpec* as a command prefix invoked with the matched "
                                    "substrings appended as arguments; its return value is the "
                                    "replacement text. (TIP 463, Tcl 9.0+)"
                                ),
                                synopsis=("regsub -command exp string cmdPrefix ?varName?",),
                                source="Tcl regsub(1)",
                            ),
                        ),
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
