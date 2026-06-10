# Enriched from F5 iRules reference documentation.
"""IP::stats -- Supplies information about the number of packets or bytes being sent or received in a given connection."""

# Introduced: BIG-IP v10+ (core iRules command) (approximate, from F5 documentation)

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IP__stats.html"


_av = make_av(_SOURCE)

_READ_EFFECT = (
    SideEffect(target=SideEffectTarget.TCP_STATE, reads=True, connection_side=ConnectionSide.BOTH),
)


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
                    synopsis="IP::stats ?pkts|bytes|in|out|age? ?in|out?",
                    arg_values={
                        0: (
                            _av("pkts", "Get packet counts.", "IP::stats pkts ?in|out?"),
                            _av("bytes", "Get byte counts.", "IP::stats bytes ?in|out?"),
                            _av("in", "Get all inbound stats.", "IP::stats in"),
                            _av("out", "Get all outbound stats.", "IP::stats out"),
                            _av("age", "Get connection age in ms.", "IP::stats age"),
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
            subcommands={
                "pkts": SubCommand(
                    name="pkts",
                    arity=Arity(0, 1),
                    detail="Get packet counts.",
                    synopsis="IP::stats pkts ?in|out?",
                    pure=True,
                    arg_values={
                        0: (
                            _av("in", "Packets received.", "IP::stats pkts in"),
                            _av("out", "Packets sent.", "IP::stats pkts out"),
                        )
                    },
                    side_effect_hints=_READ_EFFECT,
                ),
                "bytes": SubCommand(
                    name="bytes",
                    arity=Arity(0, 1),
                    detail="Get byte counts.",
                    synopsis="IP::stats bytes ?in|out?",
                    pure=True,
                    arg_values={
                        0: (
                            _av("in", "Bytes received.", "IP::stats bytes in"),
                            _av("out", "Bytes sent.", "IP::stats bytes out"),
                        )
                    },
                    side_effect_hints=_READ_EFFECT,
                ),
                "in": SubCommand(
                    name="in",
                    arity=Arity(0, 0),
                    detail="Get all inbound stats.",
                    synopsis="IP::stats in",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "out": SubCommand(
                    name="out",
                    arity=Arity(0, 0),
                    detail="Get all outbound stats.",
                    synopsis="IP::stats out",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
                "age": SubCommand(
                    name="age",
                    arity=Arity(0, 0),
                    detail="Get connection age in ms.",
                    synopsis="IP::stats age",
                    pure=True,
                    side_effect_hints=_READ_EFFECT,
                ),
            },
        )
