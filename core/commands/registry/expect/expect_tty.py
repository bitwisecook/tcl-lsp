"""expect_tty -- Expect input from the controlling tty."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register
from .expect_ import _expect_arg_roles

_SOURCE = "Expect expect_tty(1)"


@register
class ExpectTtyCommand(CommandDef):
    name = "expect_tty"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="expect_tty",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Expect input from the controlling terminal (tty).",
                synopsis=("expect_tty ?-opts? pattern body ?pattern body ...?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="expect_tty ?-opts? pattern body ?pattern body ...?",
                    options=(
                        OptionSpec(name="-re", detail="Match as regular expression."),
                        OptionSpec(name="-ex", detail="Match as exact string."),
                        OptionSpec(name="-gl", detail="Match as glob (default)."),
                        OptionSpec(name="-nocase", detail="Case-insensitive matching."),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
            arg_role_resolver=_expect_arg_roles,
        )
