# Enriched from F5 iRules reference documentation.
"""CACHE::expire -- Forces the document to be revalidated from the server."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CACHE__expire.html"


@register
class CacheExpireCommand(CommandDef):
    name = "CACHE::expire"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CACHE::expire",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Forces the document to be revalidated from the server.",
                synopsis=("CACHE::expire",),
                snippet=(
                    "Forces the document to be revalidated from the server.\n"
                    "\n"
                    "CACHE::expire\n"
                    "\n"
                    "     * Forces the document to be revalidated from the server."
                ),
                source=_SOURCE,
                examples=(
                    "when CACHE_REQUEST {\n"
                    "  if { $expire equals 1 } {\n"
                    "    CACHE::expire\n"
                    '    log "cache expire"\n'
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CACHE::expire",
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
