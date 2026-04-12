# Enriched from F5 iRules reference documentation.
"""X509::subject_public_key -- Returns the subjectXs public key of an X509 certificate."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/X509__subject_public_key.html"


_av = make_av(_SOURCE)


@register
class X509SubjectPublicKeyCommand(CommandDef):
    name = "X509::subject_public_key"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="X509::subject_public_key",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the subjectXs public key of an X509 certificate.",
                synopsis=("X509::subject_public_key (type | bits | curve_name)? CERTIFICATE",),
                snippet="Returns the subject’s public key of the specified X509 certificate.",
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_CLIENTCERT {\n"
                    "  set client_cert [SSL::cert 0]\n"
                    '  log local0. "Cert subject - [X509::subject $client_cert]"\n'
                    '  log local0. "Cert public key - [X509::subject_public_key $client_cert]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="X509::subject_public_key (type | bits | curve_name)? CERTIFICATE",
                    arg_values={
                        0: (
                            _av(
                                "type",
                                "X509::subject_public_key type",
                                "X509::subject_public_key (type | bits | curve_name)? CERTIFICATE",
                            ),
                            _av(
                                "bits",
                                "X509::subject_public_key bits",
                                "X509::subject_public_key (type | bits | curve_name)? CERTIFICATE",
                            ),
                            _av(
                                "curve_name",
                                "X509::subject_public_key curve_name",
                                "X509::subject_public_key (type | bits | curve_name)? CERTIFICATE",
                            ),
                        )
                    },
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
