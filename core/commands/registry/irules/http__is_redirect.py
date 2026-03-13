# Enriched from F5 iRules reference documentation.
"""HTTP::is_redirect -- Returns a true value if the response is a redirect."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__is_redirect.html"


@register
class HttpIsRedirectCommand(CommandDef):
    name = "HTTP::is_redirect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::is_redirect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a true value if the response is a redirect.",
                synopsis=("HTTP::is_redirect",),
                snippet=(
                    "Returns a true value if the response is a redirect. Since only\n"
                    "responses can be redirects, it does not make sense to use this command\n"
                    "in a clientside event."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    "  if { [HTTP::is_redirect] } {\n"
                    '    log local0. "Request redirected."\n'
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::is_redirect",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_STATUS,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
