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
    OptionTerminatorSpec,
    ValidationSpec,
)
from ..signatures import Arity
from ._base import register


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
            option_terminator_profiles=(
                OptionTerminatorSpec(
                    scan_start=0,
                    options_with_values=frozenset({"-start"}),
                    warn_without_terminator=True,
                ),
            ),
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
                        OptionSpec(name="--"),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            return_type=TclType.INT,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
