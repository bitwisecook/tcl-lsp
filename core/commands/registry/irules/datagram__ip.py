# Enriched from F5 iRules reference documentation.
"""DATAGRAM::ip -- Returns ip header information."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DATAGRAM__ip.html"


_av = make_av(_SOURCE)


@register
class DatagramIpCommand(CommandDef):
    name = "DATAGRAM::ip"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DATAGRAM::ip",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns ip header information.",
                synopsis=(
                    "DATAGRAM::ip (tos | ttl | flags)",
                    "DATAGRAM::ip (option | option_count) (IPV4_OPTION)?",
                ),
                snippet=(
                    "This iRules command returns ip header information.\n"
                    "\n"
                    "DATAGRAM::ip tos\n"
                    "\n"
                    "     * Returns IP header ToS as an integer value.\n"
                    "\n"
                    "DATAGRAM::ip ttl\n"
                    "\n"
                    "     * Returns IP header TTL as an integer value.\n"
                    "\n"
                    "DATAGRAM::ip flags\n"
                    "\n"
                    "     * Returns IP header flags as an integer value. The flags are from the\n"
                    "       IP datagram after IP fragment reassembly. Any MF flags that were\n"
                    "       present in indivdual fragments will not be returned. DF flag is\n"
                    "       preserved if it was set.\n"
                    "\n"
                    "DATAGRAM::ip option\n"
                    "\n"
                    "     * This command returns a Tcl list of IP options from reassembled IP\n"
                    "       datagram."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DATAGRAM::ip (tos | ttl | flags)",
                    arg_values={
                        0: (
                            _av("tos", "DATAGRAM::ip tos", "DATAGRAM::ip (tos | ttl | flags)"),
                            _av("ttl", "DATAGRAM::ip ttl", "DATAGRAM::ip (tos | ttl | flags)"),
                            _av("flags", "DATAGRAM::ip flags", "DATAGRAM::ip (tos | ttl | flags)"),
                            _av(
                                "option",
                                "DATAGRAM::ip option",
                                "DATAGRAM::ip (option | option_count) (IPV4_OPTION)?",
                            ),
                            _av(
                                "option_count",
                                "DATAGRAM::ip option_count",
                                "DATAGRAM::ip (option | option_count) (IPV4_OPTION)?",
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
