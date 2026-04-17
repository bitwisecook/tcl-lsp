# Enriched from F5 iRules reference documentation.
"""DHCPv4::yiaddr -- This command returns yiaddr(your IP) field from DHCPv4 message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DHCPv4__yiaddr.html"


@register
class Dhcpv4YiaddrCommand(CommandDef):
    name = "DHCPv4::yiaddr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DHCPv4::yiaddr",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command returns yiaddr(your IP) field from DHCPv4 message.",
                synopsis=("DHCPv4::yiaddr",),
                snippet=(
                    "This command returns yiaddr(your IP) field from DHCPv4 message\n"
                    "\n"
                    "Details (syntax):\n"
                    "DHCPv4::yiaddr"
                ),
                source=_SOURCE,
                examples=(
                    'when CLIENT_DATA {\n        log local0. "Yiaddr [DHCPv4::yiaddr]"\n    }'
                ),
                return_value="This command returns yiaddr(your IP) field from DHCPv4 message",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DHCPv4::yiaddr",
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
