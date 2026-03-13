# Enriched from F5 iRules reference documentation.
"""TCP::mss -- Returns the Maximum Segment Size (MSS) for a TCP connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__mss.html"


@register
class TcpMssCommand(CommandDef):
    name = "TCP::mss"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::mss",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the Maximum Segment Size (MSS) for a TCP connection.",
                synopsis=("TCP::mss",),
                snippet="Returns the initial connection negotiated MSS. It does not deduct bytes for any common TCP options present in data packets are not deducted. In other words, it is the minimum of the MSS options in the SYN and SYN-ACK packets, or the MSS default of 536 bytes if one packet is missing the option.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  if { [TCP::mss] >= 1280 } {\n"
                    "    COMPRESS::disable\n"
                    "  }\n"
                    "}"
                ),
                return_value="MSS in bytes.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::mss",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp"),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
