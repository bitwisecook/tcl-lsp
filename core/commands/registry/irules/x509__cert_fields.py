# Enriched from F5 iRules reference documentation.
"""X509::cert_fields -- Returns a list of X509 certificate fields to be added to HTTP headers for ModSSL behavior."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/X509__cert_fields.html"


@register
class X509CertFieldsCommand(CommandDef):
    name = "X509::cert_fields"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="X509::cert_fields",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a list of X509 certificate fields to be added to HTTP headers for ModSSL behavior.",
                synopsis=("X509::cert_fields CERTIFICATE ERROR_CODE ((hash",),
                snippet=(
                    "When given a valid certificate, returns a TCL list of field names and\n"
                    "values which can be added to the HTTP headers in order to emulate\n"
                    "ModSSL behavior. The output can be passed to 'HTTP::header insert\n"
                    "$list' as a list for insertion in the HTTP request or response."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_CLIENTCERT {\n"
                    "    if { [SSL::cert count] > 0 } {\n"
                    "        session add ssl [SSL::sessionid] [X509::cert_fields [SSL::cert 0] [SSL::verify_result] whole] $timeout\n"
                    "    }\n"
                    "}"
                ),
                return_value="Returns a list of X509 certificate fields to be added to HTTP headers.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="X509::cert_fields CERTIFICATE ERROR_CODE ((hash",
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
