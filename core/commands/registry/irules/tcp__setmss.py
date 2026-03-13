# Enriched from F5 iRules reference documentation.
"""TCP::setmss -- Sets the TCP max segment size."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__setmss.html"


@register
class TcpSetmssCommand(CommandDef):
    name = "TCP::setmss"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::setmss",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the TCP max segment size.",
                synopsis=("TCP::setmss TCP_MAX_SEGMENT_SIZE",),
                snippet=(
                    "This iRule command sets the TCP max segment size in bytes.\n"
                    "The MSS does not consider the length of any common TCP options.\n"
                    "Users should set MSS to the desired path IP packet size, minus the\n"
                    "IP header length (typically 20 bytes), minus the minimum TCP header\n"
                    "length of 20 bytes.\n"
                    "\n"
                    "TCP will automatically apply the length of common options when\n"
                    "partitioning data for delivery."
                ),
                source=_SOURCE,
                examples=(
                    "# Match clientside MSS to serverside MSS\n"
                    "when SERVER_CONNECTED {\n"
                    "    set cli_mss [clientside { TCP::mss }]\n"
                    "    set svr_mss [TCP::mss]\n"
                    "    if { $cli_mss > $svr_mss } {\n"
                    "        clientside { TCP::setmss $svr_mss }\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::setmss TCP_MAX_SEGMENT_SIZE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
