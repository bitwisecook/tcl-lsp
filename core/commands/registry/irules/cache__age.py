# Enriched from F5 iRules reference documentation.
"""CACHE::age -- Returns the age of the document in the cache."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CACHE__age.html"


@register
class CacheAgeCommand(CommandDef):
    name = "CACHE::age"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CACHE::age",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the age of the document in the cache.",
                synopsis=("CACHE::age",),
                snippet=(
                    "Returns the age of the document in the cache, in seconds.\n"
                    "\n"
                    "CACHE::age\n"
                    "\n"
                    "     * Returns the age of the document in the cache, in seconds."
                ),
                source=_SOURCE,
                examples=(
                    "when CACHE_REQUEST {\n"
                    "  if { [CACHE::age] > 60 } {\n"
                    "    CACHE::expire\n"
                    '    log local0. "Expiring content: Age > 60 seconds"\n'
                    "   }\n"
                    "}"
                ),
                return_value="Returns the age of the document in the cache, in seconds.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CACHE::age",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
