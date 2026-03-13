# Enriched from F5 iRules reference documentation.
"""TCP::dsack -- Toggles TCP duplicate selective acknowledgments (D-SACK)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__dsack.html"


@register
class TcpDsackCommand(CommandDef):
    name = "TCP::dsack"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::dsack",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Toggles TCP duplicate selective acknowledgments (D-SACK).",
                synopsis=("TCP::dsack BOOL_VALUE",),
                snippet=(
                    "Enables or disables TCP duplicate selective acknowledgements.\n"
                    "When enabled, accepts D-SACKs from remote hosts, which explicitly acknowledge duplicate packets and allow more accurate reaction to out-of-order and late packets.  See RFC3708 for details."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '    log local0. "Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port]."\n'
                    "    # Set client-side D-SACKs to enabled.\n"
                    "    clientside {\n"
                    "        TCP::dsack enable\n"
                    "    }\n"
                    "    # Set server-side D-SACKs to disabled.\n"
                    "    serverside {\n"
                    "        TCP::dsack disable\n"
                    "    }\n"
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::dsack BOOL_VALUE",
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
