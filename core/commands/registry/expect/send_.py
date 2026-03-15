"""send -- Send a string to a spawned process."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect send(1)"

_SEND_OPTIONS = (
    OptionSpec(
        name="-i",
        takes_value=True,
        value_hint="spawn_id",
        detail="Send to the specified spawn id.",
    ),
    OptionSpec(name="-raw", detail="Send without any translation."),
    OptionSpec(name="-null", detail="Send null characters."),
    OptionSpec(name="-break", detail="Send a break condition."),
    OptionSpec(name="-s", detail="Send slowly (obey send_slow parameters)."),
    OptionSpec(name="-h", detail="Send as if a human were typing (obey send_human parameters)."),
    OptionSpec(name="--", detail="End of options."),
)


@register
class SendCommand(CommandDef):
    name = "send"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="send",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Send a string to the current spawned process.",
                synopsis=("send ?-flags? string",),
                snippet=(
                    "Sends *string* to the process identified by the current "
                    "``spawn_id``. Use ``-s`` for slow sending or ``-h`` for "
                    "human-like typing."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="send ?-flags? string",
                    options=_SEND_OPTIONS,
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )
