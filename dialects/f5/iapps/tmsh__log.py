"""tmsh::log -- F5 TMSH logging and display command."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec

from ._base import _tmsh_spec, register


@register
class TmshLog(CommandDef):
    name = "tmsh::log"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::log",
            "Logs the specified message.",
            "tmsh::log <message>",
            min_args=1,
        )
