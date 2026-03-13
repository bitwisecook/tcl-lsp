# Enriched from F5 iRules reference documentation.
"""DATAGRAM::udp -- Returns UDP payload information."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DATAGRAM__udp.html"


@register
class DatagramUdpCommand(CommandDef):
    name = "DATAGRAM::udp"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DATAGRAM::udp",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns UDP payload information.",
                synopsis=(
                    "DATAGRAM::udp payload (LENGTH)?",
                    "DATAGRAM::udp payload_length",
                ),
                snippet=(
                    "This iRules command returns UDP payload information.\n"
                    "Note: throws an error if L4 protocol of the current connection is not\n"
                    "UDP\n"
                    "\n"
                    "DATAGRAM::udp payload [<size>]\n"
                    "\n"
                    "     * Returns the content of the current UDP payload. If <size> is specified and more than <size>\n"
                    "       bytes are available, only the first <size> bytes of collected data are returned.\n"
                    "\n"
                    "DATAGRAM::udp payload_length\n"
                    "\n"
                    "     * Returns the length, in bytes, of the current UDP payload."
                ),
                source=_SOURCE,
                examples=(
                    "when FLOW_INIT {\n"
                    "  if { [IP::protocol] == 17 } {\n"
                    '     log local0. "UDP Flow: [IP::client_addr] [UDP::client_port] --> [IP::local_addr] [UDP::local_port]"\n'
                    '     log local0. "UDP Payload Length = [DATAGRAM::udp payload_length] Payload: [DATAGRAM::udp payload 100]"\n'
                    "   }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DATAGRAM::udp payload (LENGTH)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"FLOW"}), also_in=frozenset({"CLIENT_DATA"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
