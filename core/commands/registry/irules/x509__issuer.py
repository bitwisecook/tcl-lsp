# Enriched from F5 iRules reference documentation.
"""X509::issuer -- Returns the issuer of an X509 certificate."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/X509__issuer.html"


@register
class X509IssuerCommand(CommandDef):
    name = "X509::issuer"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="X509::issuer",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the issuer of an X509 certificate.",
                synopsis=("X509::issuer CERTIFICATE",),
                snippet="Returns the issuer of the specified X509 certificate.",
                source=_SOURCE,
                examples=(
                    "when SERVERSSL_HANDSHAKE {\n"
                    "  set ssl_cert [SSL::cert 0]\n"
                    '  log local0. "Cert issuer - [X509::issuer $ssl_cert]"\n'
                    "}"
                ),
                return_value="Returns the issuer of an X509 certificate.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="X509::issuer CERTIFICATE",
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
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
