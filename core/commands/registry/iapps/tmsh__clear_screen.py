"""tmsh::clear_screen -- F5 TMSH logging and display command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec
from ._base import _tmsh_spec, register


@register
class TmshClearScreen(CommandDef):
    name = "tmsh::clear_screen"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::clear_screen",
            "Clears the screen.",
            "tmsh::clear_screen",
            max_args=0,
        )
