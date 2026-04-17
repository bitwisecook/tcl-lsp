"""tmsh::show -- F5 TMSH configuration command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec
from ._base import _tmsh_spec, register


@register
class TmshShow(CommandDef):
    name = "tmsh::show"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::show",
            "Runs the ``show`` command using the specified arguments.",
            "tmsh::show ?component? ?name? ?options?",
        )
