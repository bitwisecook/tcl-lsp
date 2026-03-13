# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::alert_guid -- Returns GUID that is used to identify which users have been infected with malware before the user logs in."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__alert_guid.html"


@register
class AntifraudAlertGuidCommand(CommandDef):
    name = "ANTIFRAUD::alert_guid"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::alert_guid",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns GUID that is used to identify which users have been infected with malware before the user logs in.",
                synopsis=("ANTIFRAUD::alert_guid",),
                snippet="Returns GUID that is used to identify which users have been infected with malware before the user logs in.",
                source=_SOURCE,
                examples=(
                    "when ANTIFRAUD_ALERT {\n"
                    '                log local0. "Alert GUID: [ANTIFRAUD::alert_guid]."\n'
                    "            }"
                ),
                return_value="Returns GUID that is used to identify which users have been infected with malware before the user logs in.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::alert_guid",
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
