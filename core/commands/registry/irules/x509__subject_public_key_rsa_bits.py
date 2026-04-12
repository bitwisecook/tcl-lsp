# Enriched from F5 iRules reference documentation.
"""X509::subject_public_key_RSA_bits -- Returns the size of the subjectXs public RSA key of an X509 certificate."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/X509__subject_public_key_RSA_bits.html"


@register
class X509SubjectPublicKeyRsaBitsCommand(CommandDef):
    name = "X509::subject_public_key_RSA_bits"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="X509::subject_public_key_RSA_bits",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the size of the subjectXs public RSA key of an X509 certificate.",
                synopsis=("X509::subject_public_key_RSA_bits CERTIFICATE",),
                snippet=(
                    "Returns the size, in bits, of the subject’s public RSA key of the\n"
                    "specified X509 certificate. This command is only applicable when the\n"
                    "public key type is RSA. Otherwise, the command generates an error."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  if { [info exist error_code] } {\n"
                    "    if { $error_code > 0 } {\n"
                    '      HTTP::redirect "https://some_other_site/"\n'
                    "    }\n"
                    "  }\n"
                    "}"
                ),
                return_value="Returns the size of the subject’s public RSA key of an X509 certificate.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="X509::subject_public_key_RSA_bits CERTIFICATE",
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
