# Enriched from F5 iRules reference documentation.
"""DHCPv4::xid -- This command returns xid(transaction ID) field from DHCPv4 message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DHCPv4__xid.html"


@register
class Dhcpv4XidCommand(CommandDef):
    name = "DHCPv4::xid"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DHCPv4::xid",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command returns xid(transaction ID) field from DHCPv4 message.",
                synopsis=("DHCPv4::xid",),
                snippet=(
                    "This command returns xid(transaction ID) field from DHCPv4 message\n"
                    "\n"
                    "Details (syntax):\n"
                    "DHCPv4::xid"
                ),
                source=_SOURCE,
                examples=('when CLIENT_DATA {\n        log local0. "Xid [DHCPv4::xid]"\n    }'),
                return_value="This command returns xid(transaction ID) field from DHCPv4 message",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DHCPv4::xid",
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
