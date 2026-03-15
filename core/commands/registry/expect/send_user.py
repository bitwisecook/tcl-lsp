"""send_user -- Send a string to the user (stdout)."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect send_user(1)"


@register
class SendUserCommand(CommandDef):
    name = "send_user"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="send_user",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Send a string to the user (standard output).",
                synopsis=("send_user ?-flags? string",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="send_user ?-flags? string",
                    options=(
                        OptionSpec(name="-raw", detail="Send without translation."),
                        OptionSpec(name="--", detail="End of options."),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )
