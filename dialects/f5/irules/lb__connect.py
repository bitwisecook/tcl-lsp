# Generated from F5 iRules reference documentation -- do not edit manually.
"""LB::connect -- LB::connect"""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__connect.html"


@register
class LbConnectCommand(CommandDef):
    name = "LB::connect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::connect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="LB::connect",
                synopsis=("LB::connect",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::connect",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.POOL_SELECTION,
                    writes=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
