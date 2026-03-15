"""system -- Execute a command via the system shell."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect system(1)"


@register
class SystemCommand(CommandDef):
    name = "system"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="system",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Execute a command string via the system shell (/bin/sh -c).",
                synopsis=("system args",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="system args"),),
            validation=ValidationSpec(arity=Arity(1)),
        )
