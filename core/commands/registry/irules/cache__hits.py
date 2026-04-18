# Enriched from F5 iRules reference documentation.
"""CACHE::hits -- Returns the document cache hits."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CACHE__hits.html"


@register
class CacheHitsCommand(CommandDef):
    name = "CACHE::hits"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CACHE::hits",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the document cache hits.",
                synopsis=("CACHE::hits",),
                snippet=(
                    "Returns the document cache hits.\n"
                    "\n"
                    "CACHE::hits\n"
                    "\n"
                    "     * Returns the document cache hits."
                ),
                source=_SOURCE,
                examples=(
                    "when CACHE_REQUEST {\n"
                    '  log local0. "[CACHE::hits] cache hits for document at [HTTP::uri]"\n'
                    "}"
                ),
                return_value="Returns the document cache hits.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CACHE::hits",
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
