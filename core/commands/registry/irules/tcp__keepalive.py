# Enriched from F5 iRules reference documentation.
"""TCP::keepalive -- Set/Get the TCP Keep-Alive Interval."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__keepalive.html"


@register
class TcpKeepaliveCommand(CommandDef):
    name = "TCP::keepalive"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::keepalive",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set/Get the TCP Keep-Alive Interval.",
                synopsis=("TCP::keepalive (KEEP_ALIVE_INTERVAL)?",),
                snippet="Sets or gets the number of seconds before BIG-IP sends a keep-alive packet on a TCP connection with no traffic. A value of zero indicates no keep-alive packet should be sent.",
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '    log local0. "Client: [client_addr]:[client_port] - Server: [server_addr]:[server_port]."\n'
                    "    # Set server-side keep-alive interval to 60 seconds.\n"
                    "    TCP::keepalive 60\n"
                    "}"
                ),
                return_value="TCP::keepalive without an argument returns the keep-alive interval value of a TCP connection.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::keepalive (KEEP_ALIVE_INTERVAL)?",
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
