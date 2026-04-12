"""script::help -- F5 TMSH script namespace command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec
from ._base import _tmsh_spec, register


@register
class ScriptHelp(CommandDef):
    name = "script::help"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "script::help",
            "Scripts can provide the ``script::help`` procedure for help text.",
            "script::help",
        )
