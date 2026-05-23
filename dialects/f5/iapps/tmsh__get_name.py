"""tmsh::get_name -- F5 TMSH object introspection command."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec

from ._base import _tmsh_spec, register


@register
class TmshGetName(CommandDef):
    name = "tmsh::get_name"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::get_name",
            "Returns the object identifier associated with the object.",
            "tmsh::get_name <object>",
            min_args=1,
            max_args=1,
        )
