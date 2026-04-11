# Enriched from F5 iRules reference documentation.
"""DHCPv4::drop -- This command drops DHCPv4 message silently."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DHCPv4__drop.html"


@register
class Dhcpv4DropCommand(CommandDef):
    name = "DHCPv4::drop"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DHCPv4::drop",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command drops DHCPv4 message silently.",
                synopsis=("DHCPv4::drop",),
                snippet=(
                    "This command drops DHCPv4 message silently\n\nDetails (syntax):\nDHCPv4::drop"
                ),
                source=_SOURCE,
                examples=("when CLIENT_DATA {\n        DHCPv4::drop\n    }"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DHCPv4::drop",
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
