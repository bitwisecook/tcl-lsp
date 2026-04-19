# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::alert_additional_info -- Returns or sets a list of keys and values that describes integrity parameters check failure or parameter values too long error."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_additional_info.html"


@register
class AntifraudAlertAdditionalInfoCommand(CommandDef):
    name = "ANTIFRAUD::alert_additional_info"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_additional_info",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets a list of keys and values that describes integrity parameters check failure or parameter values too long error.",
                synopsis=("ANTIFRAUD::alert_additional_info (VALUE)?",),
                snippet=(
                    "ANTIFRAUD::alert_additional_info ;\n"
                    "                Returns a list of keys and values that describes integrity parameters check failure or parameter values too long error.\n"
                    "\n"
                    "            ANTIFRAUD::alert_additional_info VALUE ;\n"
                    "                Sets a list of keys and values that describes integrity parameters check failure or parameter values too long error."
                ),
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                log local0. "original Alert additional info: [ANTIFRAUD::alert_additional_info]."\n'
                    "                ANTIFRAUD::alert_additional_info new_value\n"
                    '                log local0. "new Alert additional info: [ANTIFRAUD::alert_additional_info]."\n'
                    "            }"
                ),
                return_value="ANTIFRAUD::alert_additional_info ; Returns a list of keys and values that describes integrity parameters check failure or parameter values too long error.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_additional_info (VALUE)?",
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
