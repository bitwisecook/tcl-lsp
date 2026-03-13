# Enriched from F5 iRules reference documentation.
"""TCP::abc -- Toggles Appropriate Byte Counting."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__abc.html"


@register
class TcpAbcCommand(CommandDef):
    name = "TCP::abc"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::abc",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Toggles Appropriate Byte Counting.",
                synopsis=("TCP::abc BOOL_VALUE",),
                snippet="This command will enable or disable TCP Appropriate Byte Counting. Increases congestion window in accordance with bytes actually acknowledged, rather than allowing small acknowledgements to increase the window by an entire segment.",
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '    log local0. "Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port]."\n'
                    "    # If an HTTP connection, enable ABC on the client side and\n"
                    "    # disable ABC on the server side.\n"
                    "    if { [server_port] == 80 } {\n"
                    "        clientside {\n"
                    "            TCP::abc enable\n"
                    '            log local0. "Client MSS: [TCP::mss]"\n'
                    "        }\n"
                    "        serverside {\n"
                    "            TCP::abc disable\n"
                    '            log local0. "Server MSS: [TCP::mss]"\n'
                    "        }\n"
                    "    }\n"
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::abc BOOL_VALUE",
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
