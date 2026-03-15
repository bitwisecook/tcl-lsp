"""stty -- Set/query terminal modes."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect stty(1)"


@register
class SttyCommand(CommandDef):
    name = "stty"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="stty",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Set or query terminal modes (raw, echo, rows, columns, etc.).",
                synopsis=(
                    "stty ?args?",
                    "stty raw -echo",
                    "stty -raw echo",
                ),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="stty ?args?"),),
            validation=ValidationSpec(arity=Arity(0)),
        )
