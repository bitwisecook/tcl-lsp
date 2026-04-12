# Enriched from F5 iRules reference documentation.
"""LB::class -- Provides the name of the traffic class that matched the connection"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__class.html"


@register
class LbClassCommand(CommandDef):
    name = "LB::class"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::class",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Provides the name of the traffic class that matched the connection",
                synopsis=("LB::class",),
                snippet="Provides the name of the traffic class that matched the connection",
                source=_SOURCE,
                return_value="Return the name of the traffic class that matched the connection",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::class",
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
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
