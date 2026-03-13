# Enriched from F5 iRules reference documentation.
"""DHCPv6::peer_address -- This command returns peer address field from DHCPv6 RELAY message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DHCPv6__peer_address.html"


@register
class Dhcpv6PeerAddressCommand(CommandDef):
    name = "DHCPv6::peer_address"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DHCPv6::peer_address",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command returns peer address field from DHCPv6 RELAY message.",
                synopsis=("DHCPv6::peer_address",),
                snippet=(
                    "This command returns peer address field from DHCPv6 RELAY message\n"
                    "\n"
                    "Details (syntax):\n"
                    "DHCPv6::peer_address"
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_DATA {\n"
                    '        log local0. "Peer_address [DHCPv6::peer_address]"\n'
                    "    }"
                ),
                return_value="This command returns peer address field from DHCPv6 RELAY message",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DHCPv6::peer_address",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
