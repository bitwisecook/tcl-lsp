"""remove_nulls -- Control null-byte removal from output."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect remove_nulls(1)"


@register
class RemoveNullsCommand(CommandDef):
    name = "remove_nulls"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="remove_nulls",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Control whether null bytes are removed from spawned process output.",
                synopsis=("remove_nulls ?-d | -i spawn_id? ?0 | 1?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="remove_nulls ?-d | -i spawn_id? ?0 | 1?",
                    options=(
                        OptionSpec(name="-d", detail="Set the default."),
                        OptionSpec(
                            name="-i",
                            takes_value=True,
                            value_hint="spawn_id",
                            detail="Set for the specified spawn id.",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(0)),
        )
