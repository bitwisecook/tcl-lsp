"""strace -- Trace Expect internal statements."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect strace(1)"


@register
class StraceCommand(CommandDef):
    name = "strace"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="strace",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Trace Expect internal statements at the given detail level.",
                synopsis=("strace level",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="strace level"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )
