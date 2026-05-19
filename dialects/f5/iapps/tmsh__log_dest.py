"""tmsh::log_dest -- F5 TMSH logging and display command."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec

from ._base import _tmsh_spec, register


@register
class TmshLogDest(CommandDef):
    name = "tmsh::log_dest"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::log_dest",
            "Specifies where the system sends events.",
            "tmsh::log_dest ?destination?",
        )
