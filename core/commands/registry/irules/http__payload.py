# Enriched from F5 iRules reference documentation.
"""HTTP::payload -- Queries for or manipulates HTTP payload information."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__payload.html"


@register
class HttpPayloadCommand(CommandDef):
    name = "HTTP::payload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::payload",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Queries for or manipulates HTTP payload information.",
                synopsis=(
                    "HTTP::payload ( LENGTH | (OFFSET LENGTH) )?",
                    "HTTP::payload length",
                    "HTTP::payload rechunk",
                    "HTTP::payload unchunk",
                ),
                snippet=(
                    "Queries for or manipulates HTTP payload (content) information. With\n"
                    "this command, you can retrieve content, query for content size, or\n"
                    "replace a certain amount of content. The content does not include the\n"
                    "HTTP headers."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE_DATA {\nHTTP::respond 200 content [HTTP::payload]\n}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::payload ( LENGTH | (OFFSET LENGTH) )?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            cse_candidate=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_BODY,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                    scope=StorageScope.EVENT,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
