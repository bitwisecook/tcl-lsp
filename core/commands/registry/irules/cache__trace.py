# Enriched from F5 iRules reference documentation.
"""CACHE::trace -- Dump the list of cached objects for a HTTP profile where RAM Cache is enabled."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CACHE__trace.html"


@register
class CacheTraceCommand(CommandDef):
    name = "CACHE::trace"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CACHE::trace",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Dump the list of cached objects for a HTTP profile where RAM Cache is enabled.",
                synopsis=("CACHE::trace (MAX)?",),
                snippet=(
                    "Dump the list of cached objects for a HTTP profile where RAM Cache is\n"
                    "enabled.\n"
                    "This event will execute only if a RAM Cache profile is enabled on the\n"
                    "Virtual Server, and for objects that match the RAM Cache configuration.\n"
                    "The list will represent the size of the cache (Cache Size), number of\n"
                    "objects (Cache Count), and starting by the term Entity, it will list\n"
                    "every object:\n"
                    "  * Pos (0001), list the position of the object in the cache\n"
                    "  * Local Hits (00031/00007) indicate the number of Local Hits\n"
                    "  * Remote Hits (00031/00007) indicate the number of Remote Hits"
                ),
                source=_SOURCE,
                examples=('when RULE_INIT {\n    set static::cache ""\n}'),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CACHE::trace (MAX)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"CACHE"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
