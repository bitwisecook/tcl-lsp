# Enriched from F5 iRules reference documentation.
"""relate_client -- Sets up a related established connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/relate_client.html"


@register
class RelateClientCommand(CommandDef):
    name = "relate_client"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="relate_client",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets up a related established connection.",
                synopsis=("relate_client CONFIG",),
                snippet="Sets up a related established connection. This can be used with protocols that parse information out of a control connection and then establish a data connection based on information that was exchanged in the control connection.",
                source=_SOURCE,
                examples=(
                    "when SIP_REQUEST {\n"
                    "    # Taken from https://devcentral.f5.com/wiki/irules.Load-Balance-Outbound-SIP-Voice-Traffic-Signaling-AND-Media-with-SNAT.ashx\n"
                    "    # Pre-establish the UDP connection to allow RTP from Server -> Client (and vice versa)\n"
                    "    relate_client {\n"
                    "        proto 17\n"
                    "        clientflow $source_VLAN $destination_RTP $destination_RTP_port $source_inside $source_RTP_port\n"
                    "        serverflow $destination_VLAN $source_outside $source_RTP_port $destination_RTP $destination_RTP_port\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="relate_client CONFIG",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
