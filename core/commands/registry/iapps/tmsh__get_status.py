"""tmsh::get_status -- F5 TMSH object introspection command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec
from ._base import _tmsh_spec, register


@register
class TmshGetStatus(CommandDef):
    name = "tmsh::get_status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::get_status",
            "Returns a list of config item statuses as Tcl objects.",
            "tmsh::get_status <component> ?name? ?options?",
            min_args=1,
        )
