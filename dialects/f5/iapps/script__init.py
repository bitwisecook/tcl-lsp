"""script::init -- F5 TMSH script namespace command."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec

from ._base import _tmsh_spec, register


@register
class ScriptInit(CommandDef):
    name = "script::init"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "script::init",
            "Called before ``script::run``, ``script::help``, and ``script::tabc``.",
            "script::init",
        )
