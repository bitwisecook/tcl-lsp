# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::alert_defined_value -- Returns or sets defined (configured) value."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_defined_value.html"


@register
class AntifraudAlertDefinedValueCommand(CommandDef):
    name = "ANTIFRAUD::alert_defined_value"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_defined_value",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets defined (configured) value.",
                synopsis=("ANTIFRAUD::alert_defined_value (VALUE)?",),
                snippet=(
                    "ANTIFRAUD::alert_defined_value ;\n"
                    "                Returns defined (configured) value.\n"
                    "\n"
                    "            ANTIFRAUD::alert_defined_value VALUE ;\n"
                    "                Sets defined (configured) value."
                ),
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                log local0. "original Alert defined value: [ANTIFRAUD::alert_defined_value]."\n'
                    "                ANTIFRAUD::alert_defined_value new_value\n"
                    '                log local0. "new Alert defined value: [ANTIFRAUD::alert_defined_value]."\n'
                    "            }"
                ),
                return_value="ANTIFRAUD::alert_defined_value ; Returns defined (configured) value.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_defined_value (VALUE)?",
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
