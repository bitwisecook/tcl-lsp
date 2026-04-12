"""overlay -- Replace the Expect process with another program."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import _EXPECT_ONLY, register

_SOURCE = "Expect overlay(1)"


@register
class OverlayCommand(CommandDef):
    name = "overlay"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="overlay",
            dialects=_EXPECT_ONLY,
            hover=HoverSnippet(
                summary="Replace the Expect process with another program (exec-style).",
                synopsis=("overlay ?-# spawn_id ...? program ?args ...?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="overlay ?-# spawn_id ...? program ?args ...?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )
