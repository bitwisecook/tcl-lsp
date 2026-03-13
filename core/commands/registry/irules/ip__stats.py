# Enriched from F5 iRules reference documentation.
"""IP::stats -- Supplies information about the number of packets or bytes being sent or received in a given connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IP__stats.html"


_av = make_av(_SOURCE)


@register
class IpStatsCommand(CommandDef):
    name = "IP::stats"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IP::stats",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Supplies information about the number of packets or bytes being sent or received in a given connection.",
                synopsis=(
                    "IP::stats ((pkts ('in' | 'out')?) | (bytes ('in' | 'out')?) | in | out | age)?",
                ),
                snippet=(
                    "This command supplies information about the number of packets or bytes being sent or received in a given connection.\n"
                    "\n"
                    "IP::stats\n"
                    "Returns a list with Packets In, Packets Out, Bytes In, Bytes Out & Age\n"
                    "\n"
                    "IP::stats pkts in\n"
                    "Returns number of packets received\n"
                    "\n"
                    "IP::stats pkts out\n"
                    "Returns number of packets sent\n"
                    "\n"
                    "IP::stats pkts\n"
                    "Returns a Tcl list of packets in and packets out\n"
                    "\n"
                    "IP::stats bytes in\n"
                    "Returns number of bytes received\n"
                    "\n"
                    "IP::stats bytes out\n"
                    "Returns number of bytes sent\n"
                    "\n"
                    "IP::stats bytes\n"
                    "Returns Tcl list of bytes in and bytes out\n"
                    "\n"
                    "IP::stats age\n"
                    "Returns the age of the connection in milliseconds"
                ),
                source=_SOURCE,
                examples=(
                    "# The following example calculates and logs response time:\n"
                    "when HTTP_REQUEST {\n"
                    "    set reqAge [IP::stats age]\n"
                    "    set reqURI [HTTP::uri]\n"
                    "    set reqClient [IP::remote_addr]:[TCP::remote_port]\n"
                    "}"
                ),
                return_value="number of packets or bytes being sent or received in a given connection",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IP::stats ((pkts ('in' | 'out')?) | (bytes ('in' | 'out')?) | in | out | age)?",
                    arg_values={
                        0: (
                            _av(
                                "in",
                                "IP::stats in",
                                "IP::stats ((pkts ('in' | 'out')?) | (bytes ('in' | 'out')?) | in | out | age)?",
                            ),
                            _av(
                                "out",
                                "IP::stats out",
                                "IP::stats ((pkts ('in' | 'out')?) | (bytes ('in' | 'out')?) | in | out | age)?",
                            ),
                        )
                    },
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
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
