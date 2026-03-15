"""expect_before -- Define patterns tested before each expect."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register
from .expect_ import _expect_arg_roles

_SOURCE = "Expect expect_before(1)"


@register
class ExpectBeforeCommand(CommandDef):
    name = "expect_before"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="expect_before",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Define patterns tested before each expect command.",
                synopsis=("expect_before ?-opts? pattern body ?pattern body ...?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="expect_before ?-opts? pattern body ?pattern body ...?",
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
                        OptionSpec(name="-info", detail="Return current patterns."),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
            arg_role_resolver=_expect_arg_roles,
        )
