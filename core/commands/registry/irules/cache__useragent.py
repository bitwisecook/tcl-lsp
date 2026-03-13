# Enriched from F5 iRules reference documentation.
"""CACHE::useragent -- Overrides the useragent value used by the cache to reference the cached content."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CACHE__useragent.html"


@register
class CacheUseragentCommand(CommandDef):
    name = "CACHE::useragent"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CACHE::useragent",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Overrides the useragent value used by the cache to reference the cached content.",
                synopsis=("CACHE::useragent AGENT",),
                snippet=(
                    "By default, cached content is stored with a unique key that\n"
                    "consists of the Host header, URI, Accept-Encoding and User-Agent.\n"
                    "If the content is generated the same for multiple User-Agents,\n"
                    "this command can be used to group various User-Agent values into a single\n"
                    "group, thus minimizing duplicated cached content.\n"
                    "\n"
                    "CACHE::useragent <string>\n"
                    "\n"
                    "     * Overrides the useragent value used by the cache to store the cached\n"
                    "       content."
                ),
                source=_SOURCE,
                examples=('when HTTP_REQUEST {\n  CACHE::useragent "GENERIC-UA"\n}'),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CACHE::useragent AGENT",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
