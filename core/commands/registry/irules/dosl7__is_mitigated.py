# Enriched from F5 iRules reference documentation.
"""DOSL7::is_mitigated -- Returns TRUE if certain HTTP request was mitigated by DOSL7."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DOSL7__is_mitigated.html"


@register
class Dosl7IsMitigatedCommand(CommandDef):
    name = "DOSL7::is_mitigated"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DOSL7::is_mitigated",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns TRUE if certain HTTP request was mitigated by DOSL7.",
                synopsis=("DOSL7::is_mitigated",),
                snippet="Returns TRUE if certain HTTP request was mitigated by DOSL7.",
                source=_SOURCE,
                return_value="Returns TRUE if request was mitigated",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DOSL7::is_mitigated",
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
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
