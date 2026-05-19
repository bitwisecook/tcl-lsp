"""tmsh::get_type -- F5 TMSH object introspection command."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec

from ._base import _tmsh_spec, register


@register
class TmshGetType(CommandDef):
    name = "tmsh::get_type"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::get_type",
            "Returns the type identifier associated with the object.",
            "tmsh::get_type <object>",
            min_args=1,
            max_args=1,
        )
