# Enriched from F5 iRules reference documentation.
"""TCP::earlyrxmit -- Toggles TCP early retransmit."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__earlyrxmit.html"


@register
class TcpEarlyrxmitCommand(CommandDef):
    name = "TCP::earlyrxmit"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::earlyrxmit",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Toggles TCP early retransmit.",
                synopsis=("TCP::earlyrxmit (BOOL_VALUE)?",),
                snippet="Early retransmit allows TCP to assume a packet is lost after fewer than the standard number of duplicate ACKs, if there is no way to send new data and generate more duplicate ACKs (specified in RFC 5827).",
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '    log local0. "Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port]."\n'
                    "    # Set client-side early retransmit to enabled.\n"
                    "    clientside {\n"
                    '        log local0. "Client: earlyrxmit [TCP::earlyrxmit], enabling"\n'
                    "        TCP::earlyrxmit enable\n"
                    "    }\n"
                    "    # Set server-side early retransmit to disabled.\n"
                    "    serverside {\n"
                    '        log local0. "Server: earlyrxmit [TCP::earlyrxmit], disabling"\n'
                    "        TCP::earlyrxmit disable\n"
                    "    }\n"
                    "}"
                ),
                return_value="TCP::earlyrxmit returns whether TCP early retransmit is enabled.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::earlyrxmit (BOOL_VALUE)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
