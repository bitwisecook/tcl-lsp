# Enriched from F5 iRules reference documentation.
"""SSL::verify_result -- Gets or sets the result code for peer certificate verification."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__verify_result.html"


@register
class SslVerifyResultCommand(CommandDef):
    name = "SSL::verify_result"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::verify_result",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the result code for peer certificate verification.",
                synopsis=("SSL::verify_result (RESULT_CODE)?",),
                snippet="Gets or sets the result code for peer certificate verification. Result codes use the same values as those of OpenSSL's X509 verify_result (X509_V_ERR_*) definitions.",
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_CLIENTCERT {\n"
                    "    set cert [X509::verify_cert_error_string [SSL::verify_result]]\n"
                    "}"
                ),
                return_value="SSL::verify_result Gets the result code from peer certificate verification. The returned code uses the same values as those of OpenSSL's X509 verify_result (X509_V_ERR_) definitions.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::verify_result (RESULT_CODE)?",
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
