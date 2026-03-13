# Generated from F5 iRules reference documentation -- do not edit manually.
"""LB::select -- Forces a load balancing selection and returns the result."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__select.html"


@register
class LbSelectCommand(CommandDef):
    name = "LB::select"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::select",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Forces a load balancing selection and returns the result.",
                synopsis=("LB::select",),
                snippet="This command forces the system to make a load balancing selection based on current conditions, and returns a string in the form of a pool command that can be eval'd to activate that selection.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::select",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(flow=True),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.POOL_SELECTION,
                    writes=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
