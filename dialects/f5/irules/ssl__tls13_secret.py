# Enriched from F5 iRules reference documentation.
"""SSL::tls13_secret -- Return data about various TLS 1.3 secrets."""

# Introduced: BIG-IP v14+ (TLS 1.3) (approximate, from F5 documentation)

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
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__tls13_secret.html"


_av = make_av(_SOURCE)


@register
class SslTls13SecretCommand(CommandDef):
    name = "SSL::tls13_secret"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::tls13_secret",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Return data about various TLS 1.3 secrets.",
                synopsis=(
                    "SSL::tls13_secret client (app | hs | early)",
                    "SSL::tls13_secret server (app | hs)",
                ),
                snippet='Return TLS 1.3 session secrets. Choose which side (client or server) and which secret. "app" references the first traffic secret, "hs" -- the handshake traffic secret and "early" -- the client early traffic secret.',
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_HANDSHAKE {\n"
                    '    log local0. "ClientSSL: Client Handshake Traffic Secret [SSL::clientrandom] is -> [SSL::tls13_secret client hs]"\n'
                    '    log local0. "ClientSSL: Server Handshake Traffic Secret [SSL::clientrandom] is -> [SSL::tls13_secret server hs]"\n'
                    '    log local0. "ClientSSL: Client App Traffic Secret [SSL::clientrandom] is -> [SSL::tls13_secret client app]"\n'
                    '    log local0. "ClientSSL: Server App Traffic Secret [SSL::clientrandom] is -> [SSL::tls13_secret server app]"'
                ),
                return_value="SSL::tls13_secret client app Returns the client app secret. SSL::tls13_secret server app Returns the server app secret. SSL::tls13_secret client hs Returns the client handshake secret SSL::tls13_secret server hs Returns the server handshake secret.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::tls13_secret <side> <secret_type>",
                    arg_values={
                        0: (
                            _av(
                                "client",
                                "Client-side TLS 1.3 secret.",
                                "SSL::tls13_secret client (app | hs | early)",
                            ),
                            _av(
                                "server",
                                "Server-side TLS 1.3 secret.",
                                "SSL::tls13_secret server (app | hs)",
                            ),
                        )
                    },
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
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
            subcommands={
                "client": SubCommand(
                    name="client",
                    arity=Arity(1, 1),
                    detail="Client-side TLS 1.3 secret.",
                    synopsis="SSL::tls13_secret client (app | hs | early)",
                    pure=True,
                    arg_values={
                        0: (
                            _av(
                                "app",
                                "Client application traffic secret.",
                                "SSL::tls13_secret client app",
                            ),
                            _av(
                                "hs",
                                "Client handshake traffic secret.",
                                "SSL::tls13_secret client hs",
                            ),
                            _av(
                                "early",
                                "Client early traffic secret.",
                                "SSL::tls13_secret client early",
                            ),
                        )
                    },
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.SSL_STATE,
                            reads=True,
                            connection_side=ConnectionSide.CLIENT,
                        ),
                    ),
                ),
                "server": SubCommand(
                    name="server",
                    arity=Arity(1, 1),
                    detail="Server-side TLS 1.3 secret.",
                    synopsis="SSL::tls13_secret server (app | hs)",
                    pure=True,
                    arg_values={
                        0: (
                            _av(
                                "app",
                                "Server application traffic secret.",
                                "SSL::tls13_secret server app",
                            ),
                            _av(
                                "hs",
                                "Server handshake traffic secret.",
                                "SSL::tls13_secret server hs",
                            ),
                        )
                    },
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.SSL_STATE,
                            reads=True,
                            connection_side=ConnectionSide.SERVER,
                        ),
                    ),
                ),
            },
        )
