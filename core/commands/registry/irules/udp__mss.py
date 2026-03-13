# Enriched from F5 iRules reference documentation.
"""UDP::mss -- Returns the on-wire Maximum Segment Size (MSS) for a UDP connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/UDP__mss.html"


@register
class UdpMssCommand(CommandDef):
    name = "UDP::mss"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="UDP::mss",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the on-wire Maximum Segment Size (MSS) for a UDP connection.",
                synopsis=("UDP::mss",),
                snippet="Returns the on-wire Maximum Segment Size (MSS) for a UDP connection.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "  if { [UDP::mss] < 1000 } {\n"
                    "    pool small_req_pool\n"
                    "  } else {\n"
                    "    pool large_req_pool\n"
                    "  }\n"
                    "}"
                ),
                return_value="Returns the on-wire Maximum Segment Size (MSS) for a UDP connection",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="UDP::mss",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="udp"),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UDP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
