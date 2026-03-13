# Enriched from F5 iRules reference documentation.
"""TAP::score -- Returns or updates risk score."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TAP__score.html"


@register
class TapScoreCommand(CommandDef):
    name = "TAP::score"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TAP::score",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or updates risk score.",
                synopsis=("TAP::score (SCORE)?",),
                snippet="If score specified sets supplied score. Returns previous score.",
                source=_SOURCE,
                examples=(
                    "when TAP_REQUEST {\n"
                    "    if {    ([TAP::score] > 85) } {\n"
                    "        drop\n"
                    "    }\n"
                    "}"
                ),
                return_value="Returns an integer value from (0 to 100). If supplied score to set function returns previous score.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TAP::score (SCORE)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"TAP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
