# Enriched from F5 iRules reference documentation.
"""SSL::alpn -- Handle the ALPN TLS extension."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__alpn.html"


@register
class SslAlpnCommand(CommandDef):
    name = "SSL::alpn"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::alpn",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Handle the ALPN TLS extension.",
                synopsis=(
                    "SSL::alpn set (ARG)+",
                    "SSL::alpn",
                ),
                snippet=(
                    "Sets or retrieves the Application Layer Protocol Negotiation (ALPN) string.\n"
                    "\n"
                    "SSL::alpn\n"
                    "  Retrieve the selected ALPN string\n"
                    "\n"
                    "SSL::alpn set str1[ str2...]\n"
                    "  Set the advertised ALPN string"
                ),
                source=_SOURCE,
                examples=(
                    'when CLIENTSSL_CLIENTHELLO {\n    SSL::alpn set "spdy/1" "spdy/2" "http/2"\n}'
                ),
                return_value="SSL::alpn Returns the negotiated ALPN string SSL::alpn set ... There is no return value.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::alpn set (ARG)+",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                client_side=True, transport="tcp", profiles=frozenset({"CLIENTSSL"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
