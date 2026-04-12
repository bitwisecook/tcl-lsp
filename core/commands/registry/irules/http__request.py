# Enriched from F5 iRules reference documentation.
"""HTTP::request -- Returns the raw HTTP request headers as a string."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__request.html"


@register
class HttpRequestCommand(CommandDef):
    name = "HTTP::request"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::request",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns the raw HTTP request headers as a string.",
                synopsis=("HTTP::request",),
                snippet="Returns the raw HTTP request headers as a string.",
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    "    if {$lookup == 1 }{\n"
                    "        # collect first response (from lookup server) only\n"
                    "        HTTP::collect 1\n"
                    "    }\n"
                    "}"
                ),
                return_value="Returns the raw HTTP request headers as a string. You can access the request payload (e.g. POST data) by triggering payload collection with the [HTTP::collect] command and then using [HTTP::payload] in the [HTTP_REQUEST_DATA] event.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::request",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
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

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
