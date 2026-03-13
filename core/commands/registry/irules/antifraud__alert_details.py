# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::alert_details -- Returns or sets alert details."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_details.html"


@register
class AntifraudAlertDetailsCommand(CommandDef):
    name = "ANTIFRAUD::alert_details"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_details",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets alert details.",
                synopsis=("ANTIFRAUD::alert_details (VALUE)?",),
                snippet=(
                    "ANTIFRAUD::alert_details ;\n"
                    "                Returns alert details.\n"
                    "\n"
                    "            ANTIFRAUD::alert_details VALUE ;\n"
                    "                Sets alert details."
                ),
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                log local0. "original Alert details: [ANTIFRAUD::alert_details]."\n'
                    "                ANTIFRAUD::alert_details new_value\n"
                    '                log local0. "new Alert details: [ANTIFRAUD::alert_details]."\n'
                    "            }"
                ),
                return_value="ANTIFRAUD::alert_details ; Returns alert details.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_details (VALUE)?",
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
