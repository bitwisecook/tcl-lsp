"""tmsh::get_field_value -- F5 TMSH object introspection command."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec

from ._base import _tmsh_spec, register


@register
class TmshGetFieldValue(CommandDef):
    name = "tmsh::get_field_value"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::get_field_value",
            "Retrieves the value of the field name.",
            "tmsh::get_field_value <object> <field_name>",
            min_args=2,
            max_args=2,
        )
