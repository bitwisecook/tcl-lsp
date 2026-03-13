# Enriched from F5 iRules reference documentation.
"""UDP::release -- Allow client-side ingress to flow following a call to UDP::hold."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/UDP__release.html"


@register
class UdpReleaseCommand(CommandDef):
    name = "UDP::release"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="UDP::release",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Allow client-side ingress to flow following a call to UDP::hold.",
                synopsis=("UDP::release",),
                snippet="Called at some point after UDP::hold was called.  Unblock ingress on client side.",
                source=_SOURCE,
                examples=("when LB_SELECTED {\n    UDP::release\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="UDP::release",
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
