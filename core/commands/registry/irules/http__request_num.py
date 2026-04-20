# Scaffolded -- request_num is event-stable and HTTP namespace.
"""HTTP::request_num -- Returns the ordinal number of the current HTTP request on the connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__request_num.html"


@register
class HttpRequestNumCommand(CommandDef):
    name = "HTTP::request_num"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::request_num",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns the ordinal number of the current HTTP request on the connection.",
                synopsis=("HTTP::request_num",),
                snippet=(
                    "Returns the ordinal number of the current HTTP request on this\n"
                    "TCP connection.  The first request is 1.  Useful for detecting\n"
                    "keep-alive request boundaries."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::request_num",
                ),
            ),
            validation=ValidationSpec(arity=Arity(0, 0)),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            cse_candidate=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_HEADER,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                    scope=StorageScope.EVENT,
                ),
            ),
        )
