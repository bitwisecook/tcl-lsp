# Enriched from F5 iRules reference documentation.
"""UDP::hold -- Hold client ingress until UDP::release is called."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/UDP__hold.html"


@register
class UdpHoldCommand(CommandDef):
    name = "UDP::hold"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="UDP::hold",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Hold client ingress until UDP::release is called.",
                synopsis=("UDP::hold",),
                snippet="Hold back processing of input packets until UDP::release is called.",
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n    UDP::hold\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="UDP::hold",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="udp"),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UDP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
