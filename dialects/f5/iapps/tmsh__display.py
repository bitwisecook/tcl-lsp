"""tmsh::display -- F5 TMSH logging and display command."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec

from ._base import _tmsh_spec, register


@register
class TmshDisplay(CommandDef):
    name = "tmsh::display"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::display",
            "Provides access to the tmsh pager.",
            "tmsh::display <text>",
            min_args=1,
        )
