# Enriched from F5 iRules reference documentation.
"""AUTH::ssl_cc_ldap_status -- Returns the status from the last successful client certificate-based LDAP query."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AUTH__ssl_cc_ldap_status.html"


@register
class AuthSslCcLdapStatusCommand(CommandDef):
    name = "AUTH::ssl_cc_ldap_status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AUTH::ssl_cc_ldap_status",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the status from the last successful client certificate-based LDAP query.",
                synopsis=("AUTH::ssl_cc_ldap_status AUTH_ID",),
                snippet=(
                    "Returns the status from the last successful client certificate-based\n"
                    "LDAP query for the specified authorization session <authid>. The system\n"
                    "returns an empty string if the last successful query did not perform a\n"
                    "client certificate-based LDAP query, or if no query has yet been\n"
                    "performed. This command has been deprecated in favor of\n"
                    "AUTH::response_data.\n"
                    "\n"
                    "AUTH::ssl_cc_ldap_status <authid>\n"
                    "\n"
                    "     * Returns the status from the last successful client\n"
                    "       certificate-based LDAP query for the specified authorization\n"
                    "       session <authid>."
                ),
                source=_SOURCE,
                examples=('when RULE_INIT {\n    set tmm_auth_subscription "*"\n}'),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AUTH::ssl_cc_ldap_status AUTH_ID",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
