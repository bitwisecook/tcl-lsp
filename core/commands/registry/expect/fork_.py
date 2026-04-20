"""fork -- Fork the Expect process."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect fork(1)"


@register
class ForkCommand(CommandDef):
    name = "fork"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="fork",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Fork the Expect process, returning 0 in the child and the child pid in the parent.",
                synopsis=("fork",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="fork"),),
            validation=ValidationSpec(arity=Arity(0, 0)),
        )
