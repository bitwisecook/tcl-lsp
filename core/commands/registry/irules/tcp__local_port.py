# Enriched from F5 iRules reference documentation.
"""TCP::local_port -- Returns the local port of a TCP connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__local_port.html"


_av = make_av(_SOURCE)


@register
class TcpLocalPortCommand(CommandDef):
    name = "TCP::local_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::local_port",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns the local port of a TCP connection.",
                synopsis=("TCP::local_port (clientside | serverside)?",),
                snippet=(
                    "Returns the local port/service number of the specified side, or the current context (client or server) if there is no argument.\n"
                    "This command is equivalent to the BIG-IP 4.X variable local_port. When used\n"
                    "in a clientside context, this command returns the client-side TCP\n"
                    "destination port. When used in a serverside context, this command\n"
                    "returns the server-side TCP source port."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "  # This logs information about the TCP connections on *both* sides of the fullproxy\n"
                    '  set client_remote "[IP::client_addr]:[TCP::client_port]"\n'
                    '  set client_local  "[IP::local_addr clientside]:[TCP::local_port clientside]"\n'
                    '  set server_local  "[IP::local_addr]:[TCP::local_port]"\n'
                    '  set server_remote "[IP::server_addr]:[TCP::server_port]"\n'
                    '  log local0. "Got connection: Client($client_remote)<->($client_local)LTM($server_local)<->($server_remote)Server"\n'
                    "}"
                ),
                return_value="The local port.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::local_port (clientside | serverside)?",
                    arg_values={
                        0: (
                            _av(
                                "clientside",
                                "TCP::local_port clientside",
                                "TCP::local_port (clientside | serverside)?",
                            ),
                            _av(
                                "serverside",
                                "TCP::local_port serverside",
                                "TCP::local_port (clientside | serverside)?",
                            ),
                        )
                    },
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
                    connection_side=ConnectionSide.BOTH,
                    scope=StorageScope.CONNECTION,
                ),
            ),
        )
