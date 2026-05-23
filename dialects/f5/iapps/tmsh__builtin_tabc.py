"""tmsh::builtin_tabc -- F5 TMSH help and tab-completion command."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec

from ._base import _tmsh_spec, register


@register
class TmshBuiltinTabc(CommandDef):
    name = "tmsh::builtin_tabc"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::builtin_tabc",
            "Displays the same tab completion results as a built-in command.",
            "tmsh::builtin_tabc ?args?",
        )
