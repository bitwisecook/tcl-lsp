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
    OptionTerminatorSpec,
    ValidationSpec,
)
from ..signatures import Arity
from ._base import register


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
            option_terminator_profiles=(
                OptionTerminatorSpec(
                    scan_start=0,
                    options_with_values=frozenset({"-start"}),
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
            validation=ValidationSpec(
                arity=Arity(3),
            ),
            return_type=TclType.INT,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
