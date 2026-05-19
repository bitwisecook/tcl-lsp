"""tmsh::create -- F5 TMSH configuration command."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec

from ._base import _tmsh_spec, register


@register
class TmshCreate(CommandDef):
    name = "tmsh::create"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::create",
            "Mirrors the tmsh ``create`` command.",
            "tmsh::create <component> <name> ?options?",
            min_args=2,
        )
