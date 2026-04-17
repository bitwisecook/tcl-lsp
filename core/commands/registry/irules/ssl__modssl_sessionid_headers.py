# Enriched from F5 iRules reference documentation.
"""SSL::modssl_sessionid_headers -- Returns a list of fields for HTTP headers."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__modssl_sessionid_headers.html"


_av = make_av(_SOURCE)


@register
class SslModsslSessionidHeadersCommand(CommandDef):
    name = "SSL::modssl_sessionid_headers"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::modssl_sessionid_headers",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a list of fields for HTTP headers.",
                synopsis=("SSL::modssl_sessionid_headers (initial | current)?",),
                snippet="Returns a list of fields that the system will add to the HTTP headers, in order to emulate modssl behavior. The return type is a Tcl list; this list will be interpreted as a header-name/header-value pair by HTTP::header, for example.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    HTTP::header insert [SSL::modssl_sessionid_headers]\n"
                    "}"
                ),
                return_value='SSL::modssl_sessionid_headers Returns a header name of "SSLClientSessionId", and a header value of the session id requested by the client.',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::modssl_sessionid_headers (initial | current)?",
                    arg_values={
                        0: (
                            _av(
                                "initial",
                                "SSL::modssl_sessionid_headers initial",
                                "SSL::modssl_sessionid_headers (initial | current)?",
                            ),
                            _av(
                                "current",
                                "SSL::modssl_sessionid_headers current",
                                "SSL::modssl_sessionid_headers (initial | current)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
