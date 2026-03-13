# Enriched from F5 iRules reference documentation.
"""DHCPv6::hop_count -- This command returns hop-count field from DHCPv6 relay message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DHCPv6__hop_count.html"


@register
class Dhcpv6HopCountCommand(CommandDef):
    name = "DHCPv6::hop_count"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DHCPv6::hop_count",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command returns hop-count field from DHCPv6 relay message.",
                synopsis=("DHCPv6::hop_count",),
                snippet=(
                    "This command returns hop-count field from DHCPv6 relay message\n"
                    "\n"
                    "Details (syntax):\n"
                    "DHCPv6::hop_count"
                ),
                source=_SOURCE,
                examples=(
                    'when CLIENT_DATA {\n        log local0. "Hop-count [DHCPv6::hop_count]"\n    }'
                ),
                return_value="This command returns hop-count field from DHCPv6 relay message",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DHCPv6::hop_count",
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
