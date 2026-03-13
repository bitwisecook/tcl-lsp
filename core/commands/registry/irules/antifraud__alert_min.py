# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::alert_min -- Returns or sets variable data from client side, e.g."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_min.html"


@register
class AntifraudAlertMinCommand(CommandDef):
    name = "ANTIFRAUD::alert_min"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_min",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets variable data from client side, e.g.",
                synopsis=("ANTIFRAUD::alert_min (VALUE)?",),
                snippet=(
                    "ANTIFRAUD::alert_min ;\n"
                    "                Returns variable data from client side, e.g. forbidden added HTML element for the external_sources alert or bait signatures for the trojan_bait alert.\n"
                    "\n"
                    "            ANTIFRAUD::alert_min VALUE ;\n"
                    "                Sets variable data from client side, e.g. forbidden added HTML element for the external_sources alert or bait signatures for the trojan_bait alert."
                ),
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                if {[ANTIFRAUD::alert_type] eq "js_vhtml"} {\n'
                    '                    if {[ANTIFRAUD::alert_component] eq "external_sources"} {\n'
                    '                        log local0. "Alert forbidden added element: [ANTIFRAUD::alert_min]"\n'
                    "                    }\n"
                    '                    elseif {[ANTIFRAUD::alert_component] eq "trojan_bait"} {\n'
                    '                        log local0. "Alert bait signatures: [ANTIFRAUD::alert_min]"\n'
                    "                    }\n"
                    "                }"
                ),
                return_value="ANTIFRAUD::alert_min ; Returns variable data from client side, e.g. forbidden added HTML element for the external_sources alert or bait signatures for the trojan_bait alert.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_min (VALUE)?",
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
