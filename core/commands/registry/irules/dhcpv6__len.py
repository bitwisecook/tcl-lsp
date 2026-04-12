# Enriched from F5 iRules reference documentation.
"""DHCPv6::len -- This command returns the length of the DHCP packet length."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DHCPv6__len.html"


@register
class Dhcpv6LenCommand(CommandDef):
    name = "DHCPv6::len"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DHCPv6::len",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command returns the length of the DHCP packet length.",
                synopsis=("DHCPv6::len",),
                snippet=(
                    "This command returns the length of the DHCP packet length\n"
                    "\n"
                    "Details (syntax):\n"
                    "DHCPv6::len"
                ),
                source=_SOURCE,
                examples=('when CLIENT_DATA {\n        log local0. "Len [DHCPv6::len]"\n    }'),
                return_value="This command returns the length of the DHCP packet length",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DHCPv6::len",
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
