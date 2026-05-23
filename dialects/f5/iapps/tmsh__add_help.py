"""tmsh::add_help -- F5 TMSH help and tab-completion command."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec

from ._base import _tmsh_spec, register


@register
class TmshAddHelp(CommandDef):
    name = "tmsh::add_help"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::add_help",
            "Displays context-sensitive help when the user types ``?``.",
            "tmsh::add_help <help_data>",
            min_args=1,
        )
