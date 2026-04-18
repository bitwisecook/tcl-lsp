"""tmsh::begin_transaction -- F5 TMSH transaction command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec
from ._base import _tmsh_spec, register


@register
class TmshBeginTransaction(CommandDef):
    name = "tmsh::begin_transaction"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::begin_transaction",
            "Begins an update transaction.",
            "tmsh::begin_transaction",
            max_args=0,
        )
