# Enriched from F5 iRules reference documentation.
"""X509::verify_cert_error_string -- Returns an X509 certificate error string."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/X509__verify_cert_error_string.html"


@register
class X509VerifyCertErrorStringCommand(CommandDef):
    name = "X509::verify_cert_error_string"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="X509::verify_cert_error_string",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns an X509 certificate error string.",
                synopsis=("X509::verify_cert_error_string ERROR_CODE",),
                snippet=(
                    "Returns the same result as the OpenSSL function\n"
                    "X509_verify_cert_error_string(). Values for the <X509 verify error\n"
                    "code> argument must be the same values as those that the SSL::verify\n"
                    "result command returns."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_CLIENTCERT {\n"
                    "  set cert [SSL::cert 0]\n"
                    '  log local0. "Cert subject - [X509::subject $cert]"\n'
                    "  set error_code [SSL::verify_result]\n"
                    '  log local0. "Cert verify result - [X509::verify_cert_error_string $error_code]"\n'
                    "}"
                ),
                return_value="Returns an X509 certificate error string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="X509::verify_cert_error_string ERROR_CODE",
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
