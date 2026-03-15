"""timestamp -- Return or format a timestamp."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect timestamp(1)"


@register
class TimestampCommand(CommandDef):
    name = "timestamp"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="timestamp",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Return the current time or format a timestamp.",
                synopsis=(
                    "timestamp ?-seconds N? ?-format fmt? ?-gmt?",
                    "timestamp",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="timestamp ?-seconds N? ?-format fmt? ?-gmt?",
                    options=(
                        OptionSpec(
                            name="-seconds",
                            takes_value=True,
                            value_hint="N",
                            detail="Specify epoch seconds instead of current time.",
                        ),
                        OptionSpec(
                            name="-format",
                            takes_value=True,
                            value_hint="fmt",
                            detail="strftime-style format string.",
                        ),
                        OptionSpec(name="-gmt", detail="Use GMT instead of local time."),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
        )
