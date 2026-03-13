# Enriched from F5 iRules reference documentation.
"""TCP::remote_port -- Returns the remote TCP port number of a connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__remote_port.html"


_av = make_av(_SOURCE)


@register
class TcpRemotePortCommand(CommandDef):
    name = "TCP::remote_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::remote_port",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns the remote TCP port number of a connection.",
                synopsis=("TCP::remote_port (clientside | serverside)?",),
                snippet=(
                    "Returns the remote TCP port/service number of a TCP connection. This\n"
                    "command is equivalent to the BIG-IP 4.X variable remote_port. When used\n"
                    "in a clientside context, this command returns the client-side TCP\n"
                    "source port, and is equivalent to the TCP::client_port command.\n"
                    "When used in a serverside context, this command returns the server-side\n"
                    "TCP destination port, and is equivalent to the TCP::server_port\n"
                    "command."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "  if {[TCP::remote_port] != 443} {\n"
                    "    SSL::disable\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::remote_port (clientside | serverside)?",
                    arg_values={
                        0: (
                            _av(
                                "clientside",
                                "TCP::remote_port clientside",
                                "TCP::remote_port (clientside | serverside)?",
                            ),
                            _av(
                                "serverside",
                                "TCP::remote_port serverside",
                                "TCP::remote_port (clientside | serverside)?",
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

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(
            source={None: TaintColour.TAINTED | TaintColour.PORT},
        )
