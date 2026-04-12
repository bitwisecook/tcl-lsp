# Enriched from F5 iRules reference documentation.
"""CACHE::disabled -- Returns state of cache disable flag"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CACHE__disabled.html"


@register
class CacheDisabledCommand(CommandDef):
    name = "CACHE::disabled"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CACHE::disabled",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns state of cache disable flag",
                synopsis=("CACHE::disabled",),
                snippet=(
                    "Returns 1 for cache disabled, 0 otherwise.\n"
                    "\n"
                    "CACHE::disabled\n"
                    "\n"
                    "     * Returns true (1) or false (0) to indicate state of cache disable flag."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    "    set val [CACHE::disabled]\n"
                    '    log local0. "Cache disable state: $val"\n'
                    "}"
                ),
                return_value="Returns 0 or 1.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CACHE::disabled",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
