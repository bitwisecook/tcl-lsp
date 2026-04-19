# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::alert_id -- Returns or sets alert id."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_id.html"


@register
class AntifraudAlertIdCommand(CommandDef):
    name = "ANTIFRAUD::alert_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets alert id.",
                synopsis=("ANTIFRAUD::alert_id (VALUE)?",),
                snippet=(
                    "ANTIFRAUD::alert_id ;\n"
                    "                Returns alert id.\n"
                    "\n"
                    "            ANTIFRAUD::alert_id VALUE ;\n"
                    "                Sets alert id."
                ),
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                log local0. "original Alert ID: [ANTIFRAUD::alert_id]."\n'
                    "                ANTIFRAUD::alert_id new_value\n"
                    '                log local0. "new Alert ID: [ANTIFRAUD::alert_id]."\n'
                    "            }"
                ),
                return_value="ANTIFRAUD::alert_id ; Returns alert id.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_id (VALUE)?",
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
