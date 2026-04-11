"""tmsh::stateless -- F5 TMSH utility command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec
from ._base import _tmsh_spec, register


@register
class TmshStateless(CommandDef):
    name = "tmsh::stateless"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::stateless",
            "Modifies the behaviour of ``tmsh::create`` and ``tmsh::delete``.",
            "tmsh::stateless ?enabled?",
        )
