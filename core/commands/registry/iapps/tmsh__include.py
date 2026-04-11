"""tmsh::include -- F5 TMSH utility command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec
from ._base import _tmsh_spec, register


@register
class TmshInclude(CommandDef):
    name = "tmsh::include"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::include",
            "Runs the Tcl command ``eval`` on the specified script.",
            "tmsh::include <script_name>",
            min_args=1,
            max_args=1,
        )
