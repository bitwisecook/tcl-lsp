# Enriched from F5 iRules reference documentation.
"""DATAGRAM::ip6 -- Returns ipv6 header information."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DATAGRAM__ip6.html"


_av = make_av(_SOURCE)


@register
class DatagramIp6Command(CommandDef):
    name = "DATAGRAM::ip6"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DATAGRAM::ip6",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns ipv6 header information.",
                synopsis=(
                    "DATAGRAM::ip6 hop_limit",
                    "DATAGRAM::ip6 (option | option_count) (IPV6_OPTION)?",
                ),
                snippet=(
                    "This iRules command returns ipv6 header information.\n"
                    "Note: throws an error when used with IPv4\n"
                    "\n"
                    "DATAGRAM::ip6 hop_limit\n"
                    "\n"
                    "     * This command returns IPv6 hop limit as an integer value.\n"
                    "\n"
                    "DATAGRAM::ip6 option\n"
                    "\n"
                    "     * This command returns a Tcl list of IPv6 options from reassembled\n"
                    "       IPv6 datagram. Each option is a Tcl list with one or two values -\n"
                    "       option code (integer), and option value (byte array) if option has\n"
                    "       the value. Multiple options with the same code will be returned as\n"
                    "       separate sublists."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DATAGRAM::ip6 hop_limit",
                    arg_values={
                        0: (
                            _av(
                                "option",
                                "DATAGRAM::ip6 option",
                                "DATAGRAM::ip6 (option | option_count) (IPV6_OPTION)?",
                            ),
                            _av(
                                "option_count",
                                "DATAGRAM::ip6 option_count",
                                "DATAGRAM::ip6 (option | option_count) (IPV6_OPTION)?",
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
