# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::alert_expected_value -- Returns or sets expected (verified) value, for example in strong integrity check."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_expected_value.html"


@register
class AntifraudAlertExpectedValueCommand(CommandDef):
    name = "ANTIFRAUD::alert_expected_value"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_expected_value",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets expected (verified) value, for example in strong integrity check.",
                synopsis=("ANTIFRAUD::alert_expected_value (VALUE)?",),
                snippet=(
                    "ANTIFRAUD::alert_expected_value ;\n"
                    "                Returns expected (verified) value, for example in strong integrity check.\n"
                    "\n"
                    "            ANTIFRAUD::alert_expected_value VALUE ;\n"
                    "                Sets expected (verified) value, for example in strong integrity check."
                ),
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                log local0. "original Alert expected value: [ANTIFRAUD::alert_expected_value]."\n'
                    "                ANTIFRAUD::alert_expected_value new_value\n"
                    '                log local0. "new Alert expected value: [ANTIFRAUD::alert_expected_value]."\n'
                    "            }"
                ),
                return_value="ANTIFRAUD::alert_expected_value ; Returns expected (verified) value, for example in strong integrity check.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_expected_value (VALUE)?",
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
