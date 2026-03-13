# Enriched from F5 iRules reference documentation.
"""TCP::server_port -- Returns the remote TCP port/service number of the serverside TCP connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__server_port.html"


@register
class TcpServerPortCommand(CommandDef):
    name = "TCP::server_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::server_port",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns the remote TCP port/service number of the serverside TCP connection.",
                synopsis=("TCP::server_port",),
                snippet=(
                    "Returns the remote TCP port/service number of the serverside TCP\n"
                    "connection. This command is equivalent to the TCP::remote_port command\n"
                    "in a serverside context, and to the BIG-IP 4.x variable server_port."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "   # This logs information about:\n"
                    "   #  * the clientside part of the client<->LTM connection, and\n"
                    "   #  * the serverside part of the LTM<->server connection.\n"
                    'log local0.info "Complete connection: [IP::client_addr]:[TCP::client_port]<->LTM<->[IP::server_addr]:[TCP::server_port]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::server_port",
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
                    connection_side=ConnectionSide.SERVER,
                    scope=StorageScope.EVENT,
                ),
            ),
        )
