"""tmsh::commit_transaction -- F5 TMSH transaction command."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec
from ._base import _tmsh_spec, register


@register
class TmshCommitTransaction(CommandDef):
    name = "tmsh::commit_transaction"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::commit_transaction",
            "Runs all commands issued since the last ``tmsh::begin_transaction``.",
            "tmsh::commit_transaction",
            max_args=0,
        )
