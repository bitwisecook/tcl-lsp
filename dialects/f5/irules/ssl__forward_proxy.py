# Enriched from F5 iRules reference documentation.
"""SSL::forward_proxy -- Sets the SSL forward proxy bypass feature to bypass or intercept, or retrieves the forged certificate, or enables/disables/gets verified_handshake semantics, or mask/ignore certificate response_control for the SSL handshake or inserts a certificate extension to the certificate, or sets server certificate status."""

# Introduced: BIG-IP v12+ (SSL forward proxy / SSL Orchestrator) (approximate, from F5 documentation)

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

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__forward_proxy.html"


_av = make_av(_SOURCE)


@register
class SslForwardProxyCommand(CommandDef):
    name = "SSL::forward_proxy"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::forward_proxy",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the SSL forward proxy bypass feature to bypass or intercept, or retrieves the forged certificate, or enables/disables/gets verified_handshake semantics, or mask/ignore certificate response_control for the SSL handshake or inserts a certificate extension to the certificate, or sets server certificate status.",
                synopsis=(
                    "SSL::forward_proxy ( (policy (bypass | intercept)?) | cert)",
                    "SSL::forward_proxy verified_handshake (enable | disable) ?",
                    "SSL::forward_proxy cert response_control (ignore | mask) ?",
                    "SSL::forward_proxy extension (ARG ARG)",
                ),
                snippet="This command sets the SSL forward proxy bypass feature to bypass or intercept, or retrieves the forged certificate if the policy or cert subcommands are specified. If verified-handshake subcommand is specified, the command enables, disables or retrieves the verified_handshake behavior for the SSL handshake. If response_control subcommand is specified, the command ignore or mask the server side certificate errors while forging client certificate. If extension subcommand is specified, the command inserts an extension while forging a certificate.",
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_SERVERHELLO_SEND {\n"
                    "    log local0. 'bypassing'\n"
                    "    SSL::forward_proxy policy bypass\n"
                    "}"
                ),
                return_value='SSL::forward_proxy policy <[bypass] | [intercept]> This command sets the policy of SSL Forward Proxy Bypass feature to "bypass" or "intercept"',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::forward_proxy <subcommand> ?args?",
                    arg_values={
                        0: (
                            _av(
                                "policy",
                                "Set or get forward proxy bypass policy.",
                                "SSL::forward_proxy policy ?bypass | intercept?",
                            ),
                            _av(
                                "cert",
                                "Retrieve the forged certificate or set cert options.",
                                "SSL::forward_proxy cert ?response_control? ?status?",
                            ),
                            _av(
                                "verified_handshake",
                                "Enable, disable, or get verified handshake semantics.",
                                "SSL::forward_proxy verified_handshake ?enable | disable?",
                            ),
                            _av(
                                "extension",
                                "Insert a certificate extension while forging.",
                                "SSL::forward_proxy extension <oid> <value>",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                transport="tcp",
                profiles=frozenset({"CLIENTSSL", "SERVERSSL"}),
                also_in=frozenset({"CLIENT_ACCEPTED", "SERVER_CONNECTED"}),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
            subcommands={
                "policy": SubCommand(
                    name="policy",
                    arity=Arity(0, 1),
                    detail="Set or get forward proxy bypass policy.",
                    synopsis="SSL::forward_proxy policy ?bypass | intercept?",
                    arg_values={
                        0: (
                            _av(
                                "bypass",
                                "Bypass SSL forward proxy.",
                                "SSL::forward_proxy policy bypass",
                            ),
                            _av(
                                "intercept",
                                "Intercept SSL forward proxy.",
                                "SSL::forward_proxy policy intercept",
                            ),
                        )
                    },
                    mutator=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.SSL_STATE,
                            reads=True,
                            writes=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
                "cert": SubCommand(
                    name="cert",
                    arity=Arity(0, 2),
                    detail="Retrieve the forged certificate or set cert options.",
                    synopsis="SSL::forward_proxy cert ?response_control? ?status?",
                    arg_values={
                        0: (
                            _av(
                                "response_control",
                                "Control response to cert errors.",
                                "SSL::forward_proxy cert response_control ?ignore | mask?",
                            ),
                            _av(
                                "status",
                                "Set server certificate status.",
                                "SSL::forward_proxy cert status <value>",
                            ),
                        )
                    },
                    pure=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.SSL_STATE,
                            reads=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
                "verified_handshake": SubCommand(
                    name="verified_handshake",
                    arity=Arity(0, 1),
                    detail="Enable, disable, or get verified handshake semantics.",
                    synopsis="SSL::forward_proxy verified_handshake ?enable | disable?",
                    arg_values={
                        0: (
                            _av(
                                "enable",
                                "Enable verified handshake.",
                                "SSL::forward_proxy verified_handshake enable",
                            ),
                            _av(
                                "disable",
                                "Disable verified handshake.",
                                "SSL::forward_proxy verified_handshake disable",
                            ),
                        )
                    },
                    mutator=True,
                    side_effect_hints=(
                        SideEffect(
                            target=SideEffectTarget.SSL_STATE,
                            reads=True,
                            writes=True,
                            connection_side=ConnectionSide.BOTH,
                        ),
                    ),
                ),
                "extension": SubCommand(
                    name="extension",
                    arity=Arity(2, 2),
                    detail="Insert a certificate extension while forging.",
                    synopsis="SSL::forward_proxy extension <oid> <value>",
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
