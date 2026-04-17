# Enriched from F5 iRules reference documentation.
"""link_qos -- Returns the QoS level."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .link__qos import LinkQosCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/link_qos.html"


@register
class DeprecatedLinkQosCommand(CommandDef):
    name = "link_qos"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="link_qos",
            deprecated_replacement=LinkQosCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the QoS level.",
                synopsis=("link_qos (QOS_LEVEL)?",),
                snippet=(
                    "Returns the QoS level. The Quality of Service (QoS) standard is a means\n"
                    "by which network equipment can identify and treat traffic differently\n"
                    "based on an identifier. As traffic enters the site, the BIG-IP system\n"
                    "can apply an iRule that sends the traffic to different pools of servers\n"
                    "based on the QoS level within a packet.\n"
                    "This is a BIG-IP version 4.X variable, provided for\n"
                    "backward-compatibility. You can use the equivalent 9.X command\n"
                    "LINK::qos instead."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="link_qos (QOS_LEVEL)?",
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
