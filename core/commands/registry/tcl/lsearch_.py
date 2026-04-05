"""lsearch -- Search a list for an element matching a pattern."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register


@register
class LsearchCommand(CommandDef):
    name = "lsearch"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lsearch",
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lsearch ?options? list pattern",
                    options=(
                        OptionSpec(name="-exact"),
                        OptionSpec(name="-glob"),
                        OptionSpec(name="-regexp"),
                        OptionSpec(name="-sorted"),
                        OptionSpec(name="-all"),
                        OptionSpec(name="-inline"),
                        OptionSpec(name="-not"),
                        OptionSpec(name="-start", takes_value=True, value_hint="index"),
                        OptionSpec(name="-ascii"),
                        OptionSpec(name="-dictionary"),
                        OptionSpec(name="-integer"),
                        OptionSpec(name="-real"),
                        OptionSpec(name="-nocase"),
                        OptionSpec(name="-increasing"),
                        OptionSpec(name="-decreasing"),
                        OptionSpec(name="-bisect"),
                        OptionSpec(name="-index", takes_value=True, value_hint="index"),
                        OptionSpec(name="-subindices"),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            pure=True,
            cse_candidate=True,
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
