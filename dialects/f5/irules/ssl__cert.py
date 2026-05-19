# Enriched from F5 iRules reference documentation.
"""SSL::cert -- Returns data about an X509 SSL certificate, or sets the certificate mode."""

# Introduced: BIG-IP v9+ (core SSL iRules command) (approximate, from F5 documentation)

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour, TaintHint
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__cert.html"


_av = make_av(_SOURCE)

_READ_EFFECT = (
    SideEffect(
        target=SideEffectTarget.SSL_STATE,
        reads=True,
        connection_side=ConnectionSide.BOTH,
        scope=StorageScope.EVENT,
    ),
)


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
                synopsis=(
                    "SSL::cert <index>",
                    "SSL::cert count",
                    "SSL::cert issuer <index>",
                    "SSL::cert mode ?ignore|request|require?",
                ),
                snippet="Returns data about an X509 SSL certificate, or sets the certificate mode.",
                source=_SOURCE,
                examples=("when RULE_INIT {\n    set ::key [AES::key 128]\n}"),
                return_value="SSL::cert <index> Returns the X509 SSL certificate at the specified index in the peer certificate chain, where index is a value greater than or equal to zero. A value of zero denotes the first certificate in the chain, a value of one denotes the next, and so on.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::cert <subcommand|index> ?args?",
                    arg_values={
                        0: (
                            _av("count", "Get certificate count.", "SSL::cert count"),
                            _av(
                                "issuer",
                                "Get issuer info for cert at index.",
                                "SSL::cert issuer <index>",
                            ),
                            _av(
                                "mode",
                                "Get/set certificate mode.",
                                "SSL::cert mode ?ignore|request|require?",
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
            subcommands={
                "count": SubCommand(
                    name="count",
                    arity=Arity(0, 0),
                    detail="Get certificate count in chain.",
                    synopsis="SSL::cert count",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "issuer": SubCommand(
                    name="issuer",
                    arity=Arity(1, 1),
                    detail="Get issuer info for cert at index.",
                    synopsis="SSL::cert issuer <index>",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "mode": SubCommand(
                    name="mode",
                    arity=Arity(0, 1),
                    detail="Get/set certificate mode.",
                    synopsis="SSL::cert mode ?ignore|request|require?",
                    pure=True,
                    mutator=True,
                    arg_values={
                        0: (
                            _av("ignore", "Do not request client cert.", "SSL::cert mode ignore"),
                            _av("request", "Request client cert.", "SSL::cert mode request"),
                            _av("require", "Require client cert.", "SSL::cert mode require"),
                        )
                    },
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.SSL_STATE,
                            reads=True,
                            writes=True,
                            connection_side=ConnectionSide.BOTH,
                            scope=StorageScope.EVENT,
                        ),
                    ),
                ),
            },
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
