"""sleep -- Pause execution for a number of seconds."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity

from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect sleep(1)"


@register
class SleepCommand(CommandDef):
    name = "sleep"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="sleep",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Pause execution for the specified number of seconds.",
                synopsis=("sleep seconds",),
                snippet="Accepts integer or decimal values (e.g. ``sleep 0.5``).",
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="sleep seconds"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )
