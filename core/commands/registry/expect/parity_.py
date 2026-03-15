"""parity -- Set or query parity handling."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect parity(1)"


@register
class ParityCommand(CommandDef):
    name = "parity"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="parity",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Set or query whether parity is retained on spawned process output.",
                synopsis=("parity ?-d | -i spawn_id? ?value?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="parity ?-d | -i spawn_id? ?value?",
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
