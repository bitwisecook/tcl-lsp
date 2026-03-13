# Enriched from F5 iRules reference documentation.
"""DHCPv4::type -- This command returns type of DHCPv4 message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DHCPv4__type.html"


@register
class Dhcpv4TypeCommand(CommandDef):
    name = "DHCPv4::type"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DHCPv4::type",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command returns type of DHCPv4 message.",
                synopsis=("DHCPv4::type",),
                snippet=(
                    "This command returns type of DHCPv4 message\n\nDetails (syntax):\nDHCPv4::type"
                ),
                source=_SOURCE,
                examples=('when CLIENT_DATA {\n        log local0. "Type [DHCPv4::type]"\n    }'),
                return_value="This command returns type of DHCPv4 message",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DHCPv4::type",
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
