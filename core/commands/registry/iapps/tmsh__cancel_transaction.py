"""tmsh::cancel_transaction -- F5 TMSH transaction command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec
from ._base import _tmsh_spec, register


@register
class TmshCancelTransaction(CommandDef):
    name = "tmsh::cancel_transaction"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::cancel_transaction",
            "Cancels all commands issued since the last ``tmsh::begin_transaction``.",
            "tmsh::cancel_transaction",
            max_args=0,
        )
