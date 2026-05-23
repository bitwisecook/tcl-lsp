"""tmsh::get_field_names -- F5 TMSH object introspection command."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec

from ._base import _tmsh_spec, register


@register
class TmshGetFieldNames(CommandDef):
    name = "tmsh::get_field_names"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::get_field_names",
            "Returns a list of field names present in an object.",
            "tmsh::get_field_names <object>",
            min_args=1,
            max_args=1,
        )
