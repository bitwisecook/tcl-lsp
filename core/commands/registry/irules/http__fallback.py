# Enriched from F5 iRules reference documentation.
"""HTTP::fallback -- Specifies or overrides a fallback host specified in the HTTP profile."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__fallback.html"


@register
class HttpFallbackCommand(CommandDef):
    name = "HTTP::fallback"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::fallback",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Specifies or overrides a fallback host specified in the HTTP profile.",
                synopsis=("HTTP::fallback FALLBACK_HOST_FQDN",),
                snippet="Specifies or overrides the fallback host specified in the HTTP profile.",
                source=_SOURCE,
                examples=(
                    'when LB_FAILED {\n  HTTP::fallback "http://siteunavailable.mysite.com/"\n}'
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::fallback FALLBACK_HOST_FQDN",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_HEADER,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
