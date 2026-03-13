# Enriched from F5 iRules reference documentation.
"""AUTH::cert_issuer_credential -- Sets the peer certificate issuer credential to the value of for a future AUTH::authenticate call."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AUTH__cert_issuer_credential.html"


@register
class AuthCertIssuerCredentialCommand(CommandDef):
    name = "AUTH::cert_issuer_credential"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AUTH::cert_issuer_credential",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the peer certificate issuer credential to the value of for a future AUTH::authenticate call.",
                synopsis=("AUTH::cert_issuer_credential AUTH_ID PEER_CERTIFICATE",),
                snippet=(
                    "Sets the peer certificate issuer credential to the value of <peer\n"
                    "certificate> for a future AUTH::authenticate call. This command\n"
                    "returns an error if attempted for a standby system.\n"
                    "\n"
                    "AUTH::cert_issuer_credential authid <peer certificate>\n"
                    "\n"
                    "     * Sets the peer certificate issuer credential to <peer certificate>\n"
                    "       for a future AUTH::authenticate call."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_CLIENTCERT {\n"
                    "  set ldap_sid [AUTH::start pam $myprofilename]\n"
                    "  AUTH::cert_credential $ldap_sid [SSL::cert 0]\n"
                    "  AUTH::cert_issuer_credential $ldap_sid [SSL::cert issuer 0]\n"
                    "  AUTH::authenticate $ldap_sid\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AUTH::cert_issuer_credential AUTH_ID PEER_CERTIFICATE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
