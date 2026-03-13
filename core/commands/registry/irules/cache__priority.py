# Enriched from F5 iRules reference documentation.
"""CACHE::priority -- Adds a priority to cached documents."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CACHE__priority.html"


@register
class CachePriorityCommand(CommandDef):
    name = "CACHE::priority"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CACHE::priority",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Adds a priority to cached documents.",
                synopsis=("CACHE::priority CACHE_PRIORITY",),
                snippet=(
                    "Assigns a priority to cached documents. The priority value can be\n"
                    "between 1 and 10 inclusive. This command allows users to designate\n"
                    "documents that are costly to produce as being more important than\n"
                    "others to cache. This is particularly useful when you have a document\n"
                    "that is not requested often, but is expensive to produce (such as a\n"
                    "server-compressed document.) By increasing the priority, you are\n"
                    "increasing its likelihood of being served from the cache\n"
                    "\n"
                    "The default priority value for entries in the cache is zero (0 = cache\n"
                    "priority disabled).\n"
                    "\n"
                    "CACHE::priority <1 .."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  switch -glob [HTTP::uri] {\n"
                    '    "*.zip" -\n'
                    '    "*.tar" -\n'
                    '    "*.gz" {\n'
                    "      # set the priority to 8 for this document if it's a compressed archive\n"
                    "      CACHE::priority 8\n"
                    "    }\n"
                    '    "*.gif" -\n'
                    '    "*.jpg" -\n'
                    '    "*.png" {\n'
                    "      # set the priority to 5 for this document if it's an image\n"
                    "      CACHE::priority 5\n"
                    "    }\n"
                    '    "/mustcache*" {\n'
                    "      # Any document matching /mustcache will be set to the highest priority.\n"
                    "      CACHE::priority 10\n"
                    "    }\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CACHE::priority CACHE_PRIORITY",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
