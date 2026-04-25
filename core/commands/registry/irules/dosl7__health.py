# Enriched from F5 iRules reference documentation.
"""DOSL7::health -- Returns DOSL7 server health value for current virtual server"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DOSL7__health.html"


@register
class Dosl7HealthCommand(CommandDef):
    name = "DOSL7::health"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DOSL7::health",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns DOSL7 server health value for current virtual server",
                synopsis=("DOSL7::health",),
                snippet=("Syntax\n\nDOSL7::health"),
                source=_SOURCE,
                return_value="Return health value as integer. Lower values are good health. Bad health is value > 1^23.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DOSL7::health",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DOSL7_STATE,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
