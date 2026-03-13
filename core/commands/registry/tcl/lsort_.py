"""lsort -- Sort the elements of a list."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register


@register
class LsortCommand(CommandDef):
    name = "lsort"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lsort",
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lsort ?options? list",
                    options=(
                        OptionSpec(name="-ascii"),
                        OptionSpec(name="-dictionary"),
                        OptionSpec(name="-integer"),
                        OptionSpec(name="-real"),
                        OptionSpec(name="-nocase"),
                        OptionSpec(name="-increasing"),
                        OptionSpec(name="-decreasing"),
                        OptionSpec(name="-indices"),
                        OptionSpec(name="-unique"),
                        OptionSpec(name="-command", takes_value=True, value_hint="cmdPrefix"),
                        OptionSpec(name="-index", takes_value=True, value_hint="index"),
                        OptionSpec(name="-stride", takes_value=True, value_hint="length"),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            pure=True,
            cse_candidate=True,
            return_type=TclType.LIST,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
