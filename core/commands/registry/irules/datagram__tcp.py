# Enriched from F5 iRules reference documentation.
"""DATAGRAM::tcp -- Returns TCP header and payload information."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DATAGRAM__tcp.html"


_av = make_av(_SOURCE)


@register
class DatagramTcpCommand(CommandDef):
    name = "DATAGRAM::tcp"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DATAGRAM::tcp",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns TCP header and payload information.",
                synopsis=(
                    "DATAGRAM::tcp (flags | payload_length | window)",
                    "DATAGRAM::tcp (option | option_count) (IPV4_OPTION)?",
                    "DATAGRAM::tcp payload (LENGTH)?",
                ),
                snippet=(
                    "This iRules command returns tcp header information.\n"
                    "Note: throws an error if L4 protocol of the current connection is not\n"
                    "TCP\n"
                    "\n"
                    "DATAGRAM::tcp flags\n"
                    "\n"
                    "     * This command returns TCP header flags as an integer value.\n"
                    "\n"
                    "DATAGRAM::tcp option\n"
                    "\n"
                    "     * This command returns a Tcl list of TCP options. See\n"
                    "       DATAGRAM::ip option for details on behavior.\n"
                    "\n"
                    "DATAGRAM::tcp option [option-code]\n"
                    "\n"
                    "     * This command returns a Tcl list of TCP option values for TCP option\n"
                    "       with a given option code. See DATAGRAM::ip option for details\n"
                    "       on behavior."
                ),
                source=_SOURCE,
                examples=(
                    "when FLOW_INIT {\n"
                    "  if { [IP::protocol] == 6 } {\n"
                    '    log local0. "TCP Flow: [IP::client_addr] [TCP::client_port] --> [IP::local_addr] [TCP::local_port]"\n'
                    '    log local0. "TCP Payload Length = [DATAGRAM::tcp payload_length] Payload: [DATAGRAM::tcp payload 100]"\n'
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DATAGRAM::tcp (flags | payload_length | window)",
                    arg_values={
                        0: (
                            _av(
                                "flags",
                                "DATAGRAM::tcp flags",
                                "DATAGRAM::tcp (flags | payload_length | window)",
                            ),
                            _av(
                                "payload_length",
                                "DATAGRAM::tcp payload_length",
                                "DATAGRAM::tcp (flags | payload_length | window)",
                            ),
                            _av(
                                "window",
                                "DATAGRAM::tcp window",
                                "DATAGRAM::tcp (flags | payload_length | window)",
                            ),
                            _av(
                                "option",
                                "DATAGRAM::tcp option",
                                "DATAGRAM::tcp (option | option_count) (IPV4_OPTION)?",
                            ),
                            _av(
                                "option_count",
                                "DATAGRAM::tcp option_count",
                                "DATAGRAM::tcp (option | option_count) (IPV4_OPTION)?",
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
