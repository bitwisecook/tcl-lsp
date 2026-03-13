# Enriched from F5 iRules reference documentation.
"""SSL::forward_proxy -- Sets the SSL forward proxy bypass feature to bypass or intercept, or retrieves the forged certificate, or enables/disables/gets verified_handshake semantics, or mask/ignore certificate response_control for the SSL handshake or inserts a certificate extension to the certificate, or sets server certificate status."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
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
                    synopsis="SSL::forward_proxy ( (policy (bypass | intercept)?) | cert)",
                    arg_values={
                        0: (
                            _av(
                                "bypass",
                                "SSL::forward_proxy bypass",
                                "SSL::forward_proxy ( (policy (bypass | intercept)?) | cert)",
                            ),
                            _av(
                                "intercept",
                                "SSL::forward_proxy intercept",
                                "SSL::forward_proxy ( (policy (bypass | intercept)?) | cert)",
                            ),
                            _av(
                                "enable",
                                "SSL::forward_proxy enable",
                                "SSL::forward_proxy verified_handshake (enable | disable) ?",
                            ),
                            _av(
                                "disable",
                                "SSL::forward_proxy disable",
                                "SSL::forward_proxy verified_handshake (enable | disable) ?",
                            ),
                            _av(
                                "ignore",
                                "SSL::forward_proxy ignore",
                                "SSL::forward_proxy cert response_control (ignore | mask) ?",
                            ),
                            _av(
                                "mask",
                                "SSL::forward_proxy mask",
                                "SSL::forward_proxy cert response_control (ignore | mask) ?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                transport="tcp", profiles=frozenset({"CLIENTSSL", "SERVERSSL"})
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
