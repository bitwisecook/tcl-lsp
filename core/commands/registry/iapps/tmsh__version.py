"""tmsh::version -- F5 TMSH utility command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec
from ._base import _tmsh_spec, register


@register
class TmshVersion(CommandDef):
    name = "tmsh::version"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::version",
            "Returns the version number of the BIG-IP system as a Tcl string.",
            "tmsh::version",
            max_args=0,
        )
