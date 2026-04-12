# Enriched from F5 iRules reference documentation.
"""TCP::delayed_ack -- Toggles TCP delayed acknowledgements (ACKs)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__delayed_ack.html"


@register
class TcpDelayedAckCommand(CommandDef):
    name = "TCP::delayed_ack"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::delayed_ack",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Toggles TCP delayed acknowledgements (ACKs).",
                synopsis=("TCP::delayed_ack BOOL_VALUE",),
                snippet=(
                    "Enables or disables TCP delayed acknowledgements.\n"
                    "When enabled, minimizes acknowledgment traffic from BIG-IP by waiting 100ms for additional data to arrive, allowing aggregated ACKs. Can have negative performance implications for some remote hosts depending on their congestion control implementation."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '    log local0. "Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port]."\n'
                    "    # Set client-side delayed ACKs to enabled.\n"
                    "    clientside {\n"
                    "        TCP::delayed_ack enable\n"
                    "    }\n"
                    "    # Set server-side delayed ACKs to disabled.\n"
                    "    serverside {\n"
                    "        TCP::delayed_ack disable\n"
                    "    }\n"
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::delayed_ack BOOL_VALUE",
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
