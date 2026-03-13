# Enriched from F5 iRules reference documentation.
"""DHCPv6::reject -- This command drops the packet while sending ICMP packet about the drop reason."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DHCPv6__reject.html"


@register
class Dhcpv6RejectCommand(CommandDef):
    name = "DHCPv6::reject"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DHCPv6::reject",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command drops the packet while sending ICMP packet about the drop reason.",
                synopsis=("DHCPv6::reject",),
                snippet=(
                    "This command drops the packet while sending ICMP packet about the drop reason\n"
                    "\n"
                    "Details (syntax):\n"
                    "DHCPv6::reject"
                ),
                source=_SOURCE,
                examples=("when CLIENT_DATA {\n        DHCPv6::reject\n    }"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DHCPv6::reject",
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
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
