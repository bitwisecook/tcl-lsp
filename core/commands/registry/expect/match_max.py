"""match_max -- Set or query the maximum match buffer size."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect match_max(1)"


@register
class MatchMaxCommand(CommandDef):
    name = "match_max"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="match_max",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Set or query the maximum match buffer size.",
                synopsis=("match_max ?-d | -i spawn_id? ?size?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="match_max ?-d | -i spawn_id? ?size?",
                    options=(
                        OptionSpec(name="-d", detail="Set the default for all spawn ids."),
                        OptionSpec(
                            name="-i",
                            takes_value=True,
                            value_hint="spawn_id",
                            detail="Set for the specified spawn id.",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
        )
