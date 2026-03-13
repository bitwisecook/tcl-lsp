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
    OptionTerminatorSpec,
    ValidationSpec,
)
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl switch(1)"


@register
class SwitchCommand(CommandDef):
    name = "switch"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="switch",
            is_control_flow=True,
            never_inline_body=True,
            option_terminator_profiles=(
                OptionTerminatorSpec(
                    scan_start=0,
                    options_with_values=frozenset({"-matchvar", "-indexvar"}),
                ),
            ),
            hover=HoverSnippet(
                summary="Pattern-based branching on a subject string.",
                synopsis=("switch ?options? string pattern body ?pattern body ...?",),
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
                        OptionSpec(name="--", detail="End of options."),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(target=SideEffectTarget.UNKNOWN, connection_side=ConnectionSide.NONE),
            ),
        )
