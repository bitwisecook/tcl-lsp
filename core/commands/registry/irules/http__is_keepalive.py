# Enriched from F5 iRules reference documentation.
"""HTTP::is_keepalive -- Returns a true value if this is a Keep-Alive connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__is_keepalive.html"


@register
class HttpIsKeepaliveCommand(CommandDef):
    name = "HTTP::is_keepalive"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::is_keepalive",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a true value if this is a Keep-Alive connection.",
                synopsis=("HTTP::is_keepalive",),
                snippet="Returns a true value if this is a Keep-Alive connection.",
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n  if {[HTTP::is_keepalive]}{\n    HTTP::close\n  }\n}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::is_keepalive",
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
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
