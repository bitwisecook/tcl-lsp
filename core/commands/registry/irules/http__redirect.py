# Enriched from F5 iRules reference documentation.
"""HTTP::redirect -- Redirects an HTTP request or response to the specified URL."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__redirect.html"


@register
class HttpRedirectCommand(CommandDef):
    name = "HTTP::redirect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::redirect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Redirects an HTTP request or response to the specified URL.",
                synopsis=("HTTP::redirect REDIRECT_URL",),
                snippet=(
                    "Redirects an HTTP request or response to the specified URL. Note that\n"
                    "this command sends the response to the client immediately. Therefore,\n"
                    "you cannot specify this command multiple times in an iRule, nor can you\n"
                    "specify any other commands that modify header or content after you\n"
                    "specify this command.\n"
                    "This command will always use a 302 response code. If you wish to use a\n"
                    "different one (e.g. 301), you will need to craft a response using\n"
                    "[HTTP::respond].\n"
                    "If the client is a typical web browser, it will reflect the new URL\n"
                    "that you specify."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    "  if { [HTTP::status] == 404} {\n"
                    '    HTTP::redirect "http://www.example.com/newlocation.html"\n'
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::redirect REDIRECT_URL",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            taint_output_sink="IRULE3004",
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.RESPONSE_COMMIT,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
