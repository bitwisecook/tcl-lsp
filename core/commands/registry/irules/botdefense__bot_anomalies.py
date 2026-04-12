# Enriched from F5 iRules reference documentation.
"""BOTDEFENSE::bot_anomalies -- Returns the list of names of anomalies detected for the client that sent the current request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__bot_anomalies.html"


@register
class BotdefenseBotAnomaliesCommand(CommandDef):
    name = "BOTDEFENSE::bot_anomalies"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::bot_anomalies",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the list of names of anomalies detected for the client that sent the current request.",
                synopsis=("BOTDEFENSE::bot_anomalies",),
                snippet="Returns the list of names of anomalies detected for the client that sent the current request. Some anomalies may have been detected in previous requests of the same client and are still valid.",
                source=_SOURCE,
                examples=(
                    "when BOTDEFENSE_ACTION {\n"
                    "    foreach {anomaly} [BOTDEFENSE::bot_anomalies] {\n"
                    '        log.local0. "Found anomaly: $anomaly"\n'
                    "    }\n"
                    "}"
                ),
                return_value="Returns a list of names of all anomalies detected for the sending client. In case no anomalies found it returns an empty list.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::bot_anomalies",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"BOTDEFENSE"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
