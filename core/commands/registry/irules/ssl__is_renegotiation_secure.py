# Enriched from F5 iRules reference documentation.
"""SSL::is_renegotiation_secure -- Returns the current state of SSL Secure Renegotiation."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__is_renegotiation_secure.html"


@register
class SslIsRenegotiationSecureCommand(CommandDef):
    name = "SSL::is_renegotiation_secure"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::is_renegotiation_secure",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the current state of SSL Secure Renegotiation.",
                synopsis=("SSL::is_renegotiation_secure",),
                snippet="Returns the current state of SSL Secure Renegotiation.",
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_SERVERHELLO_SEND {\n"
                    "    set secure_renegotiation_enabled [SSL::is_renegotiation_secure]\n"
                    "}"
                ),
                return_value="SSL::is_renegotiation_secure",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::is_renegotiation_secure",
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
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
