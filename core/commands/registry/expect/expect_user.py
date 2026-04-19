"""expect_user -- Expect input from the user (stdin)."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register
from .expect_ import _expect_arg_roles

_SOURCE = "Expect expect_user(1)"


@register
class ExpectUserCommand(CommandDef):
    name = "expect_user"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="expect_user",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Expect input from the user (standard input).",
                synopsis=("expect_user ?-opts? pattern body ?pattern body ...?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="expect_user ?-opts? pattern body ?pattern body ...?",
                    options=(
                        OptionSpec(name="-re", detail="Match as regular expression."),
                        OptionSpec(name="-ex", detail="Match as exact string."),
                        OptionSpec(name="-gl", detail="Match as glob (default)."),
                        OptionSpec(name="-nocase", detail="Case-insensitive matching."),
                        OptionSpec(
                            name="-timeout",
                            takes_value=True,
                            value_hint="seconds",
                            detail="Override the timeout.",
                        ),
                        OptionSpec(name="-indices", detail="Store match indices."),
                        OptionSpec(name="-notransfer", detail="Do not consume matched output."),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
            arg_role_resolver=_expect_arg_roles,
        )
