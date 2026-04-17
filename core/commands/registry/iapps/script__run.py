"""script::run -- F5 TMSH script namespace command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec
from ._base import _tmsh_spec, register


@register
class ScriptRun(CommandDef):
    name = "script::run"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "script::run",
            "Invoked when the tmsh ``run cli script`` command is issued.",
            "script::run",
        )
