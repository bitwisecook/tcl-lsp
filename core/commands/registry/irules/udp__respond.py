# Enriched from F5 iRules reference documentation.
"""UDP::respond -- Sends data directly to a peer."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/UDP__respond.html"


@register
class UdpRespondCommand(CommandDef):
    name = "UDP::respond"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="UDP::respond",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sends data directly to a peer.",
                synopsis=("UDP::respond RESPONSE_STRING",),
                snippet=(
                    "Sends the specified data directly to the peer. This command can be used\n"
                    "to complete a protocol handshake inside an iRule."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "  set packet [binary format S {0x0000}]\n"
                    "  UDP::respond $packet\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="UDP::respond RESPONSE_STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="udp"),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UDP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
