# Enriched from F5 iRules reference documentation.
"""CACHE::uri -- Overrides the URI value used by the cache to store the cached content."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CACHE__uri.html"


@register
class CacheUriCommand(CommandDef):
    name = "CACHE::uri"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CACHE::uri",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Overrides the URI value used by the cache to store the cached content.",
                synopsis=("CACHE::uri URI_STRING",),
                snippet=(
                    "By default, cached content is stored with a unique key that consists of the Host\n"
                    "header, URI, Accept-Encoding and User-Agent. If multiple variations of the\n"
                    "same content must be cached under specific conditions (different client), you\n"
                    "can use this command to modify URI and create a unique key, thus creating\n"
                    "cached content specific to that condition. This can be used to prevent one user\n"
                    "or group's cached data from being served to different users/groups.\n"
                    "\n"
                    "If any of the key elements is rewritten at the HTTP level (HTTP_REQUEST),\n"
                    "the key uses those values, not the original values."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  set my_session_string <unique, sanitized session value>\n"
                    "  CACHE::uri [HTTP::uri]$my_session_string\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CACHE::uri URI_STRING",
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

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
