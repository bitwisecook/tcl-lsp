# Enriched from F5 iRules reference documentation.
"""SSL::cert -- Returns data about an X509 SSL certificate, or sets the certificate mode."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__cert.html"


_av = make_av(_SOURCE)


@register
class SslCertCommand(CommandDef):
    name = "SSL::cert"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::cert",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns data about an X509 SSL certificate, or sets the certificate mode.",
                synopsis=("SSL::cert (CERT_IDX | count | (issuer CERT_IDX) |",),
                snippet="Returns data about an X509 SSL certificate, or sets the certificate mode.",
                source=_SOURCE,
                examples=("when RULE_INIT {\n    set ::key [AES::key 128]\n}"),
                return_value="SSL::cert <index> Returns the X509 SSL certificate at the specified index in the peer certificate chain, where index is a value greater than or equal to zero. A value of zero denotes the first certificate in the chain, a value of one denotes the next, and so on.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::cert (CERT_IDX | count | (issuer CERT_IDX) |",
                    arg_values={
                        0: (
                            _av(
                                "issuer",
                                "SSL::cert issuer",
                                "SSL::cert (CERT_IDX | count | (issuer CERT_IDX) |",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            cse_candidate=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                    scope=StorageScope.EVENT,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
