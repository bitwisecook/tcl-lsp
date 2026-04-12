"""tmsh::get_config -- F5 TMSH object introspection command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec
from ._base import _tmsh_spec, register


@register
class TmshGetConfig(CommandDef):
    name = "tmsh::get_config"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::get_config",
            "Returns a list of configuration items as Tcl objects.",
            "tmsh::get_config <component> ?name? ?options?",
            min_args=1,
        )
