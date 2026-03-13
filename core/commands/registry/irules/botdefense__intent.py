# Enriched from F5 iRules reference documentation.
"""BOTDEFENSE::intent -- Returns the intent found for the bot that sent the current request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__intent.html"


@register
class BotdefenseIntentCommand(CommandDef):
    name = "BOTDEFENSE::intent"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::intent",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the intent found for the bot that sent the current request.",
                synopsis=("BOTDEFENSE::intent",),
                snippet="Returns the intent found for the bot that sent the current request. The intent is based on the micro-service anomaly found for that client and may have been detected in a previous request of the client, not necessarily the present request",
                source=_SOURCE,
                examples=(
                    "when BOTDEFENSE_ACTION {\n"
                    '    if {[BOTDEFENSE::intent] contains "OAT"} {\n'
                    "        BOTDEFENSE::action block\n"
                    "    }\n"
                    "}"
                ),
                return_value="Returns the intent found for the bot that sent the current request based on a micro-service anomaly found for that bot, or empty string if no intent was found. The possible intents are those available per the various micro-services types.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::intent",
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
