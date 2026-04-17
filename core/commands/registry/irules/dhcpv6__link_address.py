# Enriched from F5 iRules reference documentation.
"""DHCPv6::link_address -- This command returns link address field from DHCPv6 RELAY message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DHCPv6__link_address.html"


@register
class Dhcpv6LinkAddressCommand(CommandDef):
    name = "DHCPv6::link_address"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DHCPv6::link_address",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command returns link address field from DHCPv6 RELAY message.",
                synopsis=("DHCPv6::link_address",),
                snippet=(
                    "This command returns link address field from DHCPv6 RELAY message\n"
                    "\n"
                    "Details (syntax):\n"
                    "DHCPv6::link_address"
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_DATA {\n"
                    '        log local0. "Link_address [DHCPv6::link_address]"\n'
                    "    }"
                ),
                return_value="This command returns link address field from DHCPv6 RELAY message",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DHCPv6::link_address",
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
