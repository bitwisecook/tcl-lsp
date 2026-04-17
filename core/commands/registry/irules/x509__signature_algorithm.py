# Enriched from F5 iRules reference documentation.
"""X509::signature_algorithm -- Returns the signature algorithm of an X509 certificate."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/X509__signature_algorithm.html"


@register
class X509SignatureAlgorithmCommand(CommandDef):
    name = "X509::signature_algorithm"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="X509::signature_algorithm",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the signature algorithm of an X509 certificate.",
                synopsis=("X509::signature_algorithm CERTIFICATE",),
                snippet="Returns the signature algorithm of the specified X509 certificate.",
                source=_SOURCE,
                examples=(
                    "when SERVERSSL_HANDSHAKE {\n"
                    "    set ssl_cert [SSL::cert 0]\n"
                    '    log local0. "SIGNATURE ALGORITHM: [X509::signature_algorithm $ssl_cert]"\n'
                    "}"
                ),
                return_value="Returns the signature algorithm of an X509 certificate.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="X509::signature_algorithm CERTIFICATE",
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
