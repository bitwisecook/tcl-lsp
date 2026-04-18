"""send_error -- Send a string to stderr."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect send_error(1)"


@register
class SendErrorCommand(CommandDef):
    name = "send_error"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="send_error",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Send a string to standard error.",
                synopsis=("send_error ?-flags? string",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="send_error ?-flags? string",
                    options=(
                        OptionSpec(name="-raw", detail="Send without translation."),
                        OptionSpec(name="--", detail="End of options."),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )
