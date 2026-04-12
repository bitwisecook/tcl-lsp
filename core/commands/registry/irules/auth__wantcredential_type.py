# Enriched from F5 iRules reference documentation.
"""AUTH::wantcredential_type -- Returns an authorization session authidXs credential type."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AUTH__wantcredential_type.html"


@register
class AuthWantcredentialTypeCommand(CommandDef):
    name = "AUTH::wantcredential_type"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AUTH::wantcredential_type",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns an authorization session authidXs credential type.",
                synopsis=("AUTH::wantcredential_type AUTH_ID",),
                snippet=(
                    "Returns the authorization session authid’s credential type that the\n"
                    "system last requested (when the system generated an AUTH_WANTCREDENTIAL\n"
                    "event). The value of the <authid> argument is either username,\n"
                    "password, x509, x509_issuer, or unknown, based upon the system’s\n"
                    "assessment of the credential prompt string and style.\n"
                    "\n"
                    "AUTH::wantcredential_type <authid>\n"
                    "\n"
                    "     * Returns the authorization session authid’s credential type that the\n"
                    "       system last requested (when the system generated an\n"
                    "       AUTH_WANTCREDENTIAL event)."
                ),
                source=_SOURCE,
                examples=(
                    "when AUTH_WANTCREDENTIAL {\n"
                    '  HTTP::respond 401 "WWW-Authenticate" "Basic realm=\\"\\""\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AUTH::wantcredential_type AUTH_ID",
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
