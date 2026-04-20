# Enriched from F5 iRules reference documentation.
"""ip_tos -- F5 iRules command `ip_tos`."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .ip__tos import IpTosCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/ip_tos.html"


@register
class DeprecatedIpTosCommand(CommandDef):
    name = "ip_tos"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ip_tos",
            deprecated_replacement=IpTosCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the ToS level of a packet.",
                synopsis=("ip_tos",),
                snippet=(
                    "Returns the ToS level of a packet. The Type of Service (ToS) standard\n"
                    "is a means by which network equipment can identify and treat traffic\n"
                    "differently based on an identifier. As traffic enters the site, the\n"
                    "BIG-IP system can apply a rule that sends the traffic to different\n"
                    "pools of servers based on the ToS level within a packet.\n"
                    "This is a BIG-IP version 4.X variable, provided for\n"
                    "backward-compatibility. You can use the equivalent 9.X command\n"
                    "IP::tos instead."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ip_tos",
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
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
