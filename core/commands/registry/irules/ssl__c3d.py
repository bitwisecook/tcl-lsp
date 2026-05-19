# Enriched from F5 iRules reference documentation.
"""SSL::c3d -- Inserts a certificate extension to the C3D certificate, sets the C3D client certificate, or modifies forged certificate subject CN."""

# Introduced: BIG-IP v14+ (client certificate constrained delegation) (approximate, from F5 documentation)

from __future__ import annotations

from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__c3d.html"


_av = make_av(_SOURCE)


@register
class SslC3dCommand(CommandDef):
    name = "SSL::c3d"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::c3d",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Inserts a certificate extension to the C3D certificate, sets the C3D client certificate, or modifies forged certificate subject CN.",
                synopsis=(
                    "SSL::c3d extension (ARG ARG)",
                    "SSL::c3d cert CERTIFICATE",
                    "SSL::c3d subject (ARG ARG)",
                ),
                snippet="Inserts a certificate extension to the C3D certificate, sets the C3D client certificate, or modifies forged certificate subject CN. When subject CN is modified CN, O, OU will be converted to PrintableString where possible or UTF-8. Expected input for subject CN is in UTF-8 format.",
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_HANDSHAKE {\n"
                    '    log local0.info "CLIENTSSL_HANDSHAKE"\n'
                    '    SSL::c3d extension CP "2.16.840.1.101.2.1.11.9, cpsuri:https://localhost/test-statement/pki/cps.txt, cpsuri:https://localhost/test-statement1/pki/cps.txt;2.16.840.1.101.2.1.11.19"\n'
                    '    SSL::c3d extension SAN "DNS:*.test-client.com, IP:1.1.1.1"\n'
                    '    SSL::c3d extension 1.2.3.4 "The oid-vaule for oid 1.2.3.4"\n'
                    "    if {[SSL::cert count] > 0} {\n"
                    "        SSL::c3d subject commonName [X509::subject [SSL::cert 0] commonName]\n"
                    "    }\n"
                    "}"
                ),
                return_value="SSL::c3d extension <oid oid-value> Inserts the <oid oid-value> as an extension to C3D certificate with OID=oid and value=oid-value.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::c3d <subcommand> <args>",
                    arg_values={
                        0: (
                            _av(
                                "extension",
                                "Insert a certificate extension.",
                                "SSL::c3d extension <oid> <value>",
                            ),
                            _av(
                                "cert",
                                "Set the C3D client certificate.",
                                "SSL::c3d cert <certificate>",
                            ),
                            _av(
                                "subject",
                                "Modify forged certificate subject CN.",
                                "SSL::c3d subject <field> <value>",
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
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
            subcommands={
                "extension": SubCommand(
                    name="extension",
                    arity=Arity(2, 2),
                    detail="Insert a certificate extension.",
                    synopsis="SSL::c3d extension <oid> <value>",
                    mutator=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.SSL_STATE,
                            writes=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
                "cert": SubCommand(
                    name="cert",
                    arity=Arity(1, 1),
                    detail="Set the C3D client certificate.",
                    synopsis="SSL::c3d cert <certificate>",
                    mutator=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.SSL_STATE,
                            writes=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
                "subject": SubCommand(
                    name="subject",
                    arity=Arity(2, 2),
                    detail="Modify forged certificate subject CN.",
                    synopsis="SSL::c3d subject <field> <value>",
                    mutator=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.SSL_STATE,
                            writes=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
            },
        )
