# Enriched from F5 iRules reference documentation.
"""HTTP::status -- Returns the response status code."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__status.html"


@register
class HttpStatusCommand(CommandDef):
    name = "HTTP::status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::status",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns the response status code.",
                synopsis=("HTTP::status",),
                snippet="Returns the response status code as defined in RFC2616",
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    "  if { [HTTP::status] == 404 } {\n"
                    '    HTTP::redirect "http://www.example.com/not_found.html"\n'
                    " }\n"
                    "}"
                ),
                return_value="Returns the response status code.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::status",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            cse_candidate=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_STATUS,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                    scope=StorageScope.EVENT,
                ),
            ),
        )
