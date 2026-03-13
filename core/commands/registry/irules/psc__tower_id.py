# Enriched from F5 iRules reference documentation.
"""PSC::tower_id -- Get or set tower id."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PSC-tower-id.html"


@register
class PscTowerIdCommand(CommandDef):
    name = "PSC::tower_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PSC::tower_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set tower id.",
                synopsis=("PSC::tower_id (TOWER_ID)?",),
                snippet="The PSC::tower_id command gets the tower id or sets the tower id when the optional value is given.",
                source=_SOURCE,
                return_value="Return the tower id when no argument is given.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PSC::tower_id (TOWER_ID)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
