# Enriched from F5 iRules reference documentation.
"""DHCPv4::len -- This command returns the length of the DHCP packet length."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DHCPv4__len.html"


@register
class Dhcpv4LenCommand(CommandDef):
    name = "DHCPv4::len"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DHCPv4::len",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command returns the length of the DHCP packet length.",
                synopsis=("DHCPv4::len",),
                snippet=(
                    "This command returns the length of the DHCP packet length\n"
                    "\n"
                    "Details (syntax):\n"
                    "DHCPv4::len"
                ),
                source=_SOURCE,
                examples=('when CLIENT_DATA {\n        log local0. "Len [DHCPv4::len]"\n    }'),
                return_value="This command returns the length of the DHCP packet length",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DHCPv4::len",
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
