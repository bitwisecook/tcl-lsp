# Generated from F5 iRules reference documentation -- do not edit manually.
"""LB::src_tag -- Sets the source tag for the current request."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__src_tag.html"


@register
class LbSrcTagCommand(CommandDef):
    name = "LB::src_tag"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::src_tag",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the source tag for the current request.",
                synopsis=("LB::src_tag",),
                snippet="Sets the source tag for the current request",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::src_tag",
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
