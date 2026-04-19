"""tmsh::cd -- F5 TMSH directory command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec
from ._base import _tmsh_spec, register


@register
class TmshCd(CommandDef):
    name = "tmsh::cd"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::cd",
            "Changes the current working directory.",
            "tmsh::cd <directory>",
            min_args=1,
            max_args=1,
        )
