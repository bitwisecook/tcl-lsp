# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::alert_type -- Returns or sets alert type."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_type.html"


@register
class AntifraudAlertTypeCommand(CommandDef):
    name = "ANTIFRAUD::alert_type"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_type",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets alert type.",
                synopsis=("ANTIFRAUD::alert_type (VALUE)?",),
                snippet=(
                    "ANTIFRAUD::alert_type ;\n"
                    "                Returns alert type.\n"
                    "\n"
                    "            ANTIFRAUD::alert_type VALUE ;\n"
                    "                Sets alert type."
                ),
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                log local0. "original Alert type: [ANTIFRAUD::alert_type]."\n'
                    "                ANTIFRAUD::alert_type new_value\n"
                    '                log local0. "new Alert type: [ANTIFRAUD::alert_type]."\n'
                    "            }"
                ),
                return_value="ANTIFRAUD::alert_type ; Returns alert type.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_type (VALUE)?",
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
