"""tmsh::display_threshold -- F5 TMSH logging and display command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec
from ._base import _tmsh_spec, register


@register
class TmshDisplayThreshold(CommandDef):
    name = "tmsh::display_threshold"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::display_threshold",
            "Re-enables a display-threshold in the script.",
            "tmsh::display_threshold ?value?",
        )
