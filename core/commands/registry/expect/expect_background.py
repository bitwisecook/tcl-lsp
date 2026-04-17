"""expect_background -- Non-blocking expect."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register
from .expect_ import _expect_arg_roles

_SOURCE = "Expect expect_background(1)"


@register
class ExpectBackgroundCommand(CommandDef):
    name = "expect_background"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="expect_background",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Non-blocking expect: run pattern matching in the background.",
                synopsis=("expect_background ?-opts? pattern body ?pattern body ...?",),
                snippet=(
                    "Like ``expect`` but does not block. Whenever new data arrives "
                    "the patterns are tested and the matching body is executed."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="expect_background ?-opts? pattern body ?pattern body ...?",
                    options=(
                        OptionSpec(name="-re", detail="Match as regular expression."),
                        OptionSpec(name="-ex", detail="Match as exact string."),
                        OptionSpec(name="-gl", detail="Match as glob (default)."),
                        OptionSpec(name="-nocase", detail="Case-insensitive matching."),
                        OptionSpec(
                            name="-i",
                            takes_value=True,
                            value_hint="spawn_id",
                            detail="Specify the spawn id.",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
            arg_role_resolver=_expect_arg_roles,
        )
