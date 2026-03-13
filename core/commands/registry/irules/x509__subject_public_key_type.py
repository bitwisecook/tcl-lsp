# Enriched from F5 iRules reference documentation.
"""X509::subject_public_key_type -- Returns the subjectXs public key type of an X509 certificate."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/X509__subject_public_key_type.html"


@register
class X509SubjectPublicKeyTypeCommand(CommandDef):
    name = "X509::subject_public_key_type"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="X509::subject_public_key_type",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the subjectXs public key type of an X509 certificate.",
                synopsis=("X509::subject_public_key_type CERTIFICATE",),
                snippet=(
                    "Returns the subject’s public key type of the specified X509\n"
                    "certificate. The returned value can be either RSA, DSA, or unknown."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_CLIENTCERT {\n"
                    "  set client_cert [SSL::cert 0]\n"
                    '  log local0. "Cert subject - [X509::subject $client_cert]"\n'
                    '  log local0. "Cert public key type - [X509::subject_public_key_type $client_cert]"\n'
                    '  if { [X509::subject_public_key_type $client_cert] equals "unknown" } {\n'
                    "    SSL::verify_result 50\n"
                    "  }\n"
                    "  set error_code [SSL::verify_result]\n"
                    '  log local0. "Cert verify result - [X509::verify_cert_error_string $error_code]"\n'
                    "}"
                ),
                return_value="Returns the subject’s public key type of an X509 certificate.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="X509::subject_public_key_type CERTIFICATE",
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
