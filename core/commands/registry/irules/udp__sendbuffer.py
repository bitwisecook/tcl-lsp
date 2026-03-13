# Enriched from F5 iRules reference documentation.
"""UDP::sendbuffer -- This command can be used to set/get the maximum send buffer size (bytes) of a UDP connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/UDP__sendbuffer.html"


@register
class UdpSendbufferCommand(CommandDef):
    name = "UDP::sendbuffer"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="UDP::sendbuffer",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command can be used to set/get the maximum send buffer size (bytes) of a UDP connection.",
                synopsis=("UDP::sendbuffer (UDP_SNDBUF_SIZE)?",),
                snippet=(
                    "UDP::sendbuffer returns the maximum send buffer size (bytes) of a UDP connection.\n"
                    "UDP::sendbuffer BUFFERSIZE sets the maximum send buffer size (bytes) to specified value."
                ),
                source=_SOURCE,
                examples=(
                    "# Get/set the send buffer size of the UDP flow.\n"
                    "when CLIENT_ACCEPTED {\n"
                    '    log local0. "UDP get send buffer: [UDP::sendbuffer]"\n'
                    "    # Set the send buffer to 2,000,000 bytes\n"
                    '    log local0. "UDP set send buffer: [UDP::sendbuffer 2000000]"\n'
                    '    log local0. "UDP get send buffer: [UDP::sendbuffer]"\n'
                    "}"
                ),
                return_value="UDP::sendbuffer returns the maximum send buffer size (bytes) of a UDP connection.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="UDP::sendbuffer (UDP_SNDBUF_SIZE)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UDP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
