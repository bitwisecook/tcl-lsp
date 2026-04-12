# Enriched from F5 iRules reference documentation.
"""DHCPv6::transaction_id -- This command returns transaction id field from DHCPv6 message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DHCPv6__transation_id.html"


@register
class Dhcpv6TransactionIdCommand(CommandDef):
    name = "DHCPv6::transaction_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DHCPv6::transaction_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command returns transaction id field from DHCPv6 message.",
                synopsis=("DHCPv6::transaction_id",),
                snippet=(
                    "This command returns transaction id field from DHCPv6 message\n"
                    "\n"
                    "Details (syntax):\n"
                    "DHCPv6::transaction_id"
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_DATA {\n"
                    '        log local0. "Transaction_id [DHCPv6::transaction_id]"\n'
                    "    }"
                ),
                return_value="This command returns transaction id field from DHCPv6 message",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DHCPv6::transaction_id",
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
