"""tmsh::delete -- F5 TMSH configuration command."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec

from ._base import _tmsh_spec, register


@register
class TmshDelete(CommandDef):
    name = "tmsh::delete"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::delete",
            "Mirrors the tmsh ``delete`` command.",
            "tmsh::delete <component> <name>",
            min_args=2,
        )
