# Enriched from F5 iRules reference documentation.
"""ISTATS::remove -- Removes the given iStat entirely."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ISTATS__remove.html"


@register
class IstatsRemoveCommand(CommandDef):
    name = "ISTATS::remove"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ISTATS::remove",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Removes the given iStat entirely.",
                synopsis=("ISTATS::remove KEY",),
                snippet="Removes the given iStat entirely.",
                source=_SOURCE,
                examples='ISTATS::remove "ltm.pool /Common/mypool counter some_counter"',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ISTATS::remove KEY",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ISTATS,
                    writes=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
