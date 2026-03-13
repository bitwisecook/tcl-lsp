# Enriched from F5 iRules reference documentation.
"""HTTP::hsts -- Controls HTTP Strict Transport Security."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__hsts.html"


@register
class HttpHstsCommand(CommandDef):
    name = "HTTP::hsts"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::hsts",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Controls HTTP Strict Transport Security.",
                synopsis=(
                    "HTTP::hsts",
                    "HTTP::hsts (",
                ),
                snippet="This controls the HTTP Strict Transport Security feature options on a per-flow basis, overriding the configured values in the HTTP profile.",
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE_RELEASE {\n"
                    '    log local0.debug "HTTP Strict-Transport-Security header: [HTTP::hsts]"\n'
                    "}"
                ),
                return_value="If a value isn't given, the HTTP::hsts command will return the corresponding sub-commands currently configured value for this connection.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::hsts",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_HEADER,
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
