"""tmsh::add_tabc -- F5 TMSH help and tab-completion command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec
from ._base import _tmsh_spec, register


@register
class TmshAddTabc(CommandDef):
    name = "tmsh::add_tabc"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::add_tabc",
            "Adds tab completion datasets.",
            "tmsh::add_tabc <tabc_data>",
            min_args=1,
        )
