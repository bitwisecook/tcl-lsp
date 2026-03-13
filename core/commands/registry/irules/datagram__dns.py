# Enriched from F5 iRules reference documentation.
"""DATAGRAM::dns -- Returns DNS header information."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DATAGRAM__dns.html"


_av = make_av(_SOURCE)


@register
class DatagramDnsCommand(CommandDef):
    name = "DATAGRAM::dns"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DATAGRAM::dns",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns DNS header information.",
                synopsis=(
                    "DATAGRAM::dns (id | qr | opcode | qdcount | ancount | nscount | arcount)",
                ),
                snippet=(
                    "This iRules command returns DNS header information.\n"
                    "  Note: L4 protocol of the packet must be either TCP or UDP for this command\n"
                    "        to work. Also, the L4 port must be equal to the dns port (typically port 53).\n"
                    "\n"
                    "DATAGRAM::dns id\n"
                    "\n"
                    "     * Returns DNS header 16-bit ‘identification’ field as an integer value.\n"
                    "\n"
                    "DATAGRAM::dns qr\n"
                    "\n"
                    "     * Returns DNS header ‘query/response’ as a boolean value.\n"
                    "       0 indicates a ‘query’ and 1 indicates a ‘response’.\n"
                    "\n"
                    "DATAGRAM::dns opcode\n"
                    "\n"
                    "     * Returns DNS header ‘opcode’ as a string."
                ),
                source=_SOURCE,
                examples=(
                    "when FLOW_INIT {\n"
                    "  if { [IP::protocol] == 6 } {\n"
                    '    log local0. "TCP Payload Length = [DATAGRAM::tcp payload_length] Payload: [DATAGRAM::tcp payload 100]"\n'
                    '    log local0. "DNS Header fields ID: [DATAGRAM::dns id] QR: [DATAGRAM::dns qr] OPCODE: [DATAGRAM::dns opcode] QDCOUNT: [DATAGRAM::dns qdcount]"\n'
                    "  }\n"
                    "  if { [IP::protocol] == 17 } {\n"
                    '    log local0. "UDP Payload Length = [DATAGRAM::udp payload_length] Payload: [DATAGRAM::udp payload 100]"'
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DATAGRAM::dns (id | qr | opcode | qdcount | ancount | nscount | arcount)",
                    arg_values={
                        0: (
                            _av(
                                "id",
                                "DATAGRAM::dns id",
                                "DATAGRAM::dns (id | qr | opcode | qdcount | ancount | nscount | arcount)",
                            ),
                            _av(
                                "qr",
                                "DATAGRAM::dns qr",
                                "DATAGRAM::dns (id | qr | opcode | qdcount | ancount | nscount | arcount)",
                            ),
                            _av(
                                "opcode",
                                "DATAGRAM::dns opcode",
                                "DATAGRAM::dns (id | qr | opcode | qdcount | ancount | nscount | arcount)",
                            ),
                            _av(
                                "qdcount",
                                "DATAGRAM::dns qdcount",
                                "DATAGRAM::dns (id | qr | opcode | qdcount | ancount | nscount | arcount)",
                            ),
                            _av(
                                "ancount",
                                "DATAGRAM::dns ancount",
                                "DATAGRAM::dns (id | qr | opcode | qdcount | ancount | nscount | arcount)",
                            ),
                            _av(
                                "nscount",
                                "DATAGRAM::dns nscount",
                                "DATAGRAM::dns (id | qr | opcode | qdcount | ancount | nscount | arcount)",
                            ),
                            _av(
                                "arcount",
                                "DATAGRAM::dns arcount",
                                "DATAGRAM::dns (id | qr | opcode | qdcount | ancount | nscount | arcount)",
                            ),
                        )
                    },
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
