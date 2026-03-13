# Enriched from F5 iRules reference documentation.
"""X509::whole -- Returns an X509 certificate in PEM format."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/X509__whole.html"


@register
class X509WholeCommand(CommandDef):
    name = "X509::whole"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="X509::whole",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns an X509 certificate in PEM format.",
                synopsis=("X509::whole CERTIFICATE",),
                snippet="Returns the specified X509 certificate, in its entirety, in PEM format.",
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_CLIENTCERT {\n"
                    "  set client_cert [SSL::cert 0]\n"
                    '  log local0. "[X509::whole $client_cert]"\n'
                    "}"
                ),
                return_value="Returns an X509 certificate in PEM format.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="X509::whole CERTIFICATE",
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
