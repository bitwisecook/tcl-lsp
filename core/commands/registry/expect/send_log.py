"""send_log -- Send a string to the log file only."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect send_log(1)"


@register
class SendLogCommand(CommandDef):
    name = "send_log"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="send_log",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Send a string to the log file only (not to the process or user).",
                synopsis=("send_log ?--? string",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="send_log ?--? string",
                    options=(OptionSpec(name="--", detail="End of options."),),
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )
