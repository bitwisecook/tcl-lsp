"""exp_pid -- Return the process id of a spawned process."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect exp_pid(1)"


@register
class ExpPidCommand(CommandDef):
    name = "exp_pid"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="exp_pid",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Return the process id of a spawned process.",
                synopsis=("exp_pid ?-i spawn_id?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="exp_pid ?-i spawn_id?",
                    options=(
                        OptionSpec(
                            name="-i",
                            takes_value=True,
                            value_hint="spawn_id",
                            detail="Query the specified spawn id.",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
            pure=True,
        )
