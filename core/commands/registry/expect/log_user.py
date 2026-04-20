"""log_user -- Control logging of send/expect output to stdout."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect log_user(1)"


@register
class LogUserCommand(CommandDef):
    name = "log_user"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="log_user",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Control whether send/expect output is logged to stdout.",
                synopsis=(
                    "log_user -info",
                    "log_user 0|1",
                ),
                snippet="With ``1`` (default), output is sent to stdout. With ``0``, output is suppressed.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="log_user ?-info | 0 | 1?",
                    options=(OptionSpec(name="-info", detail="Return current setting."),),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0, 1)),
        )
