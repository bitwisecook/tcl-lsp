# Enriched from F5 iRules reference documentation.
"""DHCPv6::drop -- This command drops DHCPv6 message silently."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DHCPv6__drop.html"


@register
class Dhcpv6DropCommand(CommandDef):
    name = "DHCPv6::drop"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DHCPv6::drop",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command drops DHCPv6 message silently.",
                synopsis=("DHCPv6::drop",),
                snippet=(
                    "This command drops DHCPv6 message silently\n\nDetails (syntax):\nDHCPv6::drop"
                ),
                source=_SOURCE,
                examples=("when CLIENT_DATA {\n        DHCPv6::drop\n    }"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DHCPv6::drop",
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
