"""tmsh::pwd -- F5 TMSH directory command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec
from ._base import _tmsh_spec, register


@register
class TmshPwd(CommandDef):
    name = "tmsh::pwd"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::pwd",
            "Returns the current working directory.",
            "tmsh::pwd",
            max_args=0,
        )
