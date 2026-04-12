# Enriched from F5 iRules reference documentation.
"""SSL::cert_constraint -- Inserts cert constraint information to the certificate."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__cert_constraint.html"


@register
class SslCertConstraintCommand(CommandDef):
    name = "SSL::cert_constraint"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::cert_constraint",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Inserts cert constraint information to the certificate.",
                synopsis=("SSL::cert_constraint (ARG ARG)",),
                snippet="Inserts a certificate extension to the certificate.",
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_HANDSHAKE {\n"
                    '    log local0.info "CLIENTSSL_HANDSHAKE"\n'
                    '    SSL::cert_constraint 1.2.3.4.5 "This is the oid-value of 1.2.3.4.5"\n'
                    "}"
                ),
                return_value="SSL::cert_constraint <oid oid-value> Inserts the <oid oid-value> as an extension with OID=oid and value=oid-value to the certificate.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::cert_constraint (ARG ARG)",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                client_side=True, transport="tcp", profiles=frozenset({"CLIENTSSL"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
