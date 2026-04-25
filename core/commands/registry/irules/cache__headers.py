# Enriched from F5 iRules reference documentation.
"""CACHE::headers -- Returns the HTTP headers of the object in the cache."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CACHE__headers.html"


@register
class CacheHeadersCommand(CommandDef):
    name = "CACHE::headers"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CACHE::headers",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the HTTP headers of the object in the cache.",
                synopsis=("CACHE::headers",),
                snippet=(
                    "Returns the HTTP headers of the object in the cache.\n"
                    "If CACHE::header is used to manipulate the response headers prior to calling CACHE::headers, the modifications will not be reflected by CACHE::headers.\n"
                    "\n"
                    "CACHE::headers\n"
                    "\n"
                    "     * Returns the HTTP headers of the object in the cache as TCL Name / value pairs list."
                ),
                source=_SOURCE,
                examples=(
                    "when CACHE_RESPONSE {\n"
                    "  # log all  HTTP headers sent in cache response.\n"
                    "  log local0. [CACHE::headers]\n"
                    "}"
                ),
                return_value="Returns the HTTP headers of the object in the cache as TCL Name / value pairs list.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CACHE::headers",
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
