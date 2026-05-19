"""tmsh::list -- F5 TMSH configuration command."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec

from ._base import _tmsh_spec, register


@register
class TmshList(CommandDef):
    name = "tmsh::list"

    @classmethod
    def spec(cls) -> CommandSpec:
        return _tmsh_spec(
            "tmsh::list",
            "Runs the ``list`` command using the specified arguments.",
            "tmsh::list ?component? ?name? ?options?",
        )
