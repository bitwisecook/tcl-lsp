"""tmsh::modify -- F5 TMSH configuration command."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec

from ._base import _tmsh_spec, register


@register
class TmshModify(CommandDef):
    name = "tmsh::modify"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::modify",
            "Runs the ``modify`` command using the specified arguments.",
            "tmsh::modify <component> <name> ?options?",
            min_args=2,
        )
