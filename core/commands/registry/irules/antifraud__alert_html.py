# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::alert_html -- For js_vhtml alert: returns or sets the whole HTML in an escaped base64 format."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_html.html"


@register
class AntifraudAlertHtmlCommand(CommandDef):
    name = "ANTIFRAUD::alert_html"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_html",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="For js_vhtml alert: returns or sets the whole HTML in an escaped base64 format.",
                synopsis=("ANTIFRAUD::alert_html (VALUE)?",),
                snippet=(
                    "ANTIFRAUD::alert_html ;\n"
                    "                For js_vhtml alert: returns the whole HTML in an escaped base64 format.\n"
                    "\n"
                    "            ANTIFRAUD::alert_html VALUE ;\n"
                    "                For client side alerts: sets the whole HTML in an escaped base64 format."
                ),
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                log local0. "original Alert HTML: [ANTIFRAUD::alert_html]."\n'
                    "                ANTIFRAUD::alert_html new_value\n"
                    '                log local0. "new Alert HTML: [ANTIFRAUD::alert_html]."\n'
                    "            }"
                ),
                return_value="ANTIFRAUD::alert_html ; For js_vhtml alert: returns the whole HTML in an escaped base64 format.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_html (VALUE)?",
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
