# Enriched from F5 iRules reference documentation.
"""UDP::payload -- Returns the content or length of the current UDP payload."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/UDP__payload.html"


@register
class UdpPayloadCommand(CommandDef):
    name = "UDP::payload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="UDP::payload",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the content or length of the current UDP payload.",
                synopsis=(
                    "UDP::payload (LENGTH | (OFFSET LENGTH))?",
                    "UDP::payload length",
                    "UDP::payload replace OFFSET LENGTH UDP_PAYLOAD",
                ),
                snippet=(
                    "Returns the content or length of the current UDP payload.\n"
                    "Notice that, unlike TCP, there is no need to trigger a collect, and there is no\n"
                    "corresponding release. Moreover, this command is valid not only in\n"
                    "CLIENT_DATA and SERVER_DATA, but may be invoked within\n"
                    "CLIENT_ACCEPTED. In that case, it will evaluate to the data\n"
                    "contained in the segment that triggered the CLIENT_ACCEPTED event\n"
                    "- though not necessarily every segment in a UDP stream (see\n"
                    "CLIENT_ACCEPTED event description for more details)."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_DATA {\n"
                    "  # empty payload entirely so there is no packet to send to the server\n"
                    '  UDP::payload replace 0 [UDP::payload length] ""\n'
                    "\n"
                    "  # craft a string to hold our  packet data, 0x01 0x00 0x00 0x00 0x02 0x00 0x000x00 0x03 0x00 0x00 0x00\n"
                    "  set packetdata [binary format i1i1i1 1 2 3 ]\n"
                    "\n"
                    "  # then fill payload with our own data from arbitrary length string called packetdata to send to the server"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="UDP::payload (LENGTH | (OFFSET LENGTH))?",
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
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
