# Enriched from F5 iRules reference documentation.
"""CACHE::payload -- Returns the HTTP payload of the cache response."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CACHE__payload.html"


@register
class CachePayloadCommand(CommandDef):
    name = "CACHE::payload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CACHE::payload",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the HTTP payload of the cache response.",
                synopsis=("CACHE::payload",),
                snippet=(
                    "Returns the HTTP payload of the cache response.\n"
                    "\n"
                    "CACHE::payload\n"
                    "\n"
                    "     * Returns the HTTP payload of the cache response."
                ),
                source=_SOURCE,
                examples=("when CACHE_RESPONSE {\n  set payload [CACHE::payload]\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CACHE::payload",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"CACHE"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
