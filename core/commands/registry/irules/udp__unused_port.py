# Enriched from F5 iRules reference documentation.
"""UDP::unused_port -- Returns an unused UDP port for the specified IP tuple."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/UDP__unused_port.html"


@register
class UdpUnusedPortCommand(CommandDef):
    name = "UDP::unused_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="UDP::unused_port",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns an unused UDP port for the specified IP tuple.",
                synopsis=("UDP::unused_port REMOTE_ADDR REMOTE_PORT LOCAL_ADDR",),
                snippet="Returns an unused UDP port for the specified IP tuple.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "  set port [UDP::unused_port [IP::remote_addr] [UDP::remote_port] [IP::local_addr]]\n"
                    '  UDP::respond "Next unused port: $port"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="UDP::unused_port REMOTE_ADDR REMOTE_PORT LOCAL_ADDR",
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
