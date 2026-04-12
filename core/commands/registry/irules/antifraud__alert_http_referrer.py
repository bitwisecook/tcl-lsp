# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::alert_http_referrer -- Returns or sets alert HTTP referrer."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_http_referrer.html"


@register
class AntifraudAlertHttpReferrerCommand(CommandDef):
    name = "ANTIFRAUD::alert_http_referrer"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_http_referrer",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets alert HTTP referrer.",
                synopsis=("ANTIFRAUD::alert_http_referrer (VALUE)?",),
                snippet=(
                    "ANTIFRAUD::alert_http_referrer ;\n"
                    "                Returns alert HTTP referrer.\n"
                    "\n"
                    "            ANTIFRAUD::alert_http_referrer VALUE ;\n"
                    "                Sets alert HTTP referrer."
                ),
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                log local0. "original Alert HTTP referrer: [ANTIFRAUD::alert_http_referrer]."\n'
                    "                ANTIFRAUD::alert_http_referrer new_value\n"
                    '                log local0. "new Alert HTTP referrer: [ANTIFRAUD::alert_http_referrer]."\n'
                    "            }"
                ),
                return_value="ANTIFRAUD::alert_http_referrer ; Returns alert HTTP referrer.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_http_referrer (VALUE)?",
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
