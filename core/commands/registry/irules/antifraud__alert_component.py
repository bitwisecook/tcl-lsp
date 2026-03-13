# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::alert_component -- Returns or sets error type according to alert_type."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_component.html"


@register
class AntifraudAlertComponentCommand(CommandDef):
    name = "ANTIFRAUD::alert_component"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_component",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets error type according to alert_type.",
                synopsis=("ANTIFRAUD::alert_component (VALUE)?",),
                snippet=(
                    "ANTIFRAUD::alert_component ;\n"
                    "                Returns error type according to alert_type.\n"
                    "\n"
                    "            ANTIFRAUD::alert_component VALUE ;\n"
                    "                Sets error type according to alert_type."
                ),
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                log local0. "original Alert component: [ANTIFRAUD::alert_component]."\n'
                    "                ANTIFRAUD::alert_component new_value\n"
                    '                log local0. "new Alert component: [ANTIFRAUD::alert_component]."\n'
                    "            }"
                ),
                return_value="ANTIFRAUD::alert_component ; Returns error type according to alert_type.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_component (VALUE)?",
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
