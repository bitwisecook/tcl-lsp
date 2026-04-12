# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::alert_score -- Returns or sets alert severity."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_score.html"


@register
class AntifraudAlertScoreCommand(CommandDef):
    name = "ANTIFRAUD::alert_score"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_score",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets alert severity.",
                synopsis=("ANTIFRAUD::alert_score (VALUE)?",),
                snippet=(
                    "ANTIFRAUD::alert_score ;\n"
                    "                Returns alert severity.\n"
                    "\n"
                    "            ANTIFRAUD::alert_score VALUE ;\n"
                    "                Sets alert severity."
                ),
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                log local0. "original Alert score: [ANTIFRAUD::alert_score]."\n'
                    "                ANTIFRAUD::alert_score new_value\n"
                    '                log local0. "new Alert score: [ANTIFRAUD::alert_score]."\n'
                    "            }"
                ),
                return_value="ANTIFRAUD::alert_score ; Returns alert severity.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_score (VALUE)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ANTIFRAUD"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
