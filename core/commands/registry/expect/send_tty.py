"""send_tty -- Send a string to the controlling tty."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect send_tty(1)"


@register
class SendTtyCommand(CommandDef):
    name = "send_tty"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="send_tty",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Send a string to the controlling terminal (tty).",
                synopsis=("send_tty ?-flags? string",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="send_tty ?-flags? string",
                    options=(
                        OptionSpec(name="-raw", detail="Send without translation."),
                        OptionSpec(name="--", detail="End of options."),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )
