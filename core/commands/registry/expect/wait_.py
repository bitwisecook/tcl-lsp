"""wait -- Wait for a spawned process to terminate."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect wait(1)"


@register
class WaitCommand(CommandDef):
    name = "wait"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="wait",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Wait for a spawned process to terminate.",
                synopsis=("wait ?-i spawn_id? ?-nowait?",),
                snippet=(
                    "Returns a list of four integers: pid, spawn id, OS error, "
                    "and exit status (or -1 0 0 status on success)."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="wait ?-i spawn_id? ?-nowait?",
                    options=(
                        OptionSpec(
                            name="-i",
                            takes_value=True,
                            value_hint="spawn_id",
                            detail="Wait for the specified spawn id.",
                        ),
                        OptionSpec(name="-nowait", detail="Non-blocking wait."),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
        )
