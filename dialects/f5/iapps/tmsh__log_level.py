"""tmsh::log_level -- F5 TMSH logging and display command."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec

from ._base import _tmsh_spec, register


@register
class TmshLogLevel(CommandDef):
    name = "tmsh::log_level"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::log_level",
            "Specifies the default severity level.",
            "tmsh::log_level ?level?",
        )
