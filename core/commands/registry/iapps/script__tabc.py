"""script::tabc -- F5 TMSH script namespace command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec
from ._base import _tmsh_spec, register


@register
class ScriptTabc(CommandDef):
    name = "script::tabc"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "script::tabc",
            "A script may provide ``script::tabc`` for tab completion.",
            "script::tabc",
        )
