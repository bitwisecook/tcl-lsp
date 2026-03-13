# Enriched from F5 iRules reference documentation.
"""TCP::client_port -- Returns the client port of the TCP connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__client_port.html"


@register
class TcpClientPortCommand(CommandDef):
    name = "TCP::client_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::client_port",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns the client port of the TCP connection.",
                synopsis=("TCP::client_port",),
                snippet=(
                    "Returns the TCP port/service number of the clientside TCP\n"
                    "connection. This command is equivalent to the TCP::remote_port\n"
                    "command in a clientside context, and to the BIG-IP 4.x variable\n"
                    "client_port."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "   # This logs information about:\n"
                    "   #  * the clientside part of the client<->LTM connection, and\n"
                    "   #  * the serverside part of the LTM<->server connection.\n"
                    '   log local0.info "Complete connection: [IP::client_addr]:[TCP::client_port]<->LTM<->[IP::server_addr]:[TCP::server_port]"\n'
                    "}"
                ),
                return_value="The port advertised by the client. Even on SERVER events, it still returns the client port from the clientside.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::client_port",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp"),
            cse_candidate=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                    scope=StorageScope.CONNECTION,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(
            source={Arity(0, 0): TaintColour.TAINTED | TaintColour.PORT},
        )
