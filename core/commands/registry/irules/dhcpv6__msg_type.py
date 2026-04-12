# Enriched from F5 iRules reference documentation.
"""DHCPv6::msg_type -- This command returns message type field from DHCPv6 message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DHCPv6__msg_type.html"


@register
class Dhcpv6MsgTypeCommand(CommandDef):
    name = "DHCPv6::msg_type"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DHCPv6::msg_type",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command returns message type field from DHCPv6 message.",
                synopsis=("DHCPv6::msg_type",),
                snippet=(
                    "This command returns message type field from DHCPv6 message\n"
                    "\n"
                    "Details (syntax):\n"
                    "DHCPv6::msg_type"
                ),
                source=_SOURCE,
                examples=(
                    'when CLIENT_DATA {\n        log local0. "Msg_type [DHCPv6::msg_type]"\n    }'
                ),
                return_value="This command returns message type field from DHCPv6 message",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DHCPv6::msg_type",
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
