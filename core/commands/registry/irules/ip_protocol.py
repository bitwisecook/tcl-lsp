# Enriched from F5 iRules reference documentation.
"""ip_protocol -- Returns the IP protocol value."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .ip__protocol import IpProtocolCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/ip_protocol.html"


@register
class DeprecatedIpProtocolCommand(CommandDef):
    name = "ip_protocol"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ip_protocol",
            deprecated_replacement=IpProtocolCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the IP protocol value.",
                synopsis=("ip_protocol",),
                snippet=(
                    "Returns the IP protocol value. This is a BIG-IP 4.X variable, provided\n"
                    "for backward-compatibility. You can use the command IP::protocol\n"
                    "instead."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ip_protocol",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
