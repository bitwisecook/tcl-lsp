# Enriched from F5 iRules reference documentation.
"""redirect -- Redirects an HTTP request to the specific location."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .http__redirect import HttpRedirectCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/redirect.html"


@register
class RedirectCommand(CommandDef):
    name = "redirect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="redirect",
            deprecated_replacement=HttpRedirectCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Redirects an HTTP request to the specific location.",
                synopsis=("redirect to HOST_URI",),
                snippet=(
                    "Redirects an HTTP request to a specific location. The location can be\n"
                    "either a host name or a URI. This is a BIG-IP 4.X statement, provided\n"
                    "for backward compatibility. You can use the equivalent 9.X command\n"
                    "HTTP::redirect instead."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    # HTTP::redirect, HTTP::host and HTTP::uri should be used instead\n"
                    '    redirect to "https://[http_host][http_uri]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="redirect to HOST_URI",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.RESPONSE_COMMIT,
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
