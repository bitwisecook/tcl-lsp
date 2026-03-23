"""tmsh::builtin_help -- F5 TMSH help and tab-completion command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec
from ._base import _tmsh_spec, register


@register
class TmshBuiltinHelp(CommandDef):
    name = "tmsh::builtin_help"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::builtin_help",
            "Presents the same results as typing ``?`` while entering a tmsh command.",
            "tmsh::builtin_help ?args?",
        )
