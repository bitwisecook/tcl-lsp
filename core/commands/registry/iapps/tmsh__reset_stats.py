"""tmsh::reset_stats -- F5 TMSH utility command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec
from ._base import _tmsh_spec, register


@register
class TmshResetStats(CommandDef):
    name = "tmsh::reset_stats"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::reset_stats",
            "Runs the ``reset-stats`` command using the specified arguments.",
            "tmsh::reset_stats ?component? ?name?",
        )
