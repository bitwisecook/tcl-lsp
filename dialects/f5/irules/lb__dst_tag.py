# Generated from F5 iRules reference documentation -- do not edit manually.
"""LB::dst_tag -- Set the destination tag for the current request."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__dst_tag.html"


@register
class LbDstTagCommand(CommandDef):
    name = "LB::dst_tag"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::dst_tag",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set the destination tag for the current request.",
                synopsis=("LB::dst_tag",),
                snippet="Set the destination tag for the current request",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::dst_tag",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.POOL_SELECTION,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
