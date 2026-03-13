# Enriched from F5 iRules reference documentation.
"""BOTDEFENSE::bot_name -- Returns the name assigned to the detected bot, browser or mobile application."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__bot_name.html"


@register
class BotdefenseBotNameCommand(CommandDef):
    name = "BOTDEFENSE::bot_name"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::bot_name",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the name assigned to the detected bot, browser or mobile application.",
                synopsis=("BOTDEFENSE::bot_name",),
                snippet="Returns the name assigned to the detected bot, browser or mobile application. The name is derived from the detected signature if detected, or the User-Agent string in combination with the detected anomalies.",
                source=_SOURCE,
                examples=(
                    "# EXAMPLE: Log the Bot name and Device ID of the client, upon each request, if it is known.\n"
                    "when BOTDEFENSE_ACTION {\n"
                    '    log local0.info "Bot [BOTDEFENSE::bot_name] with Device ID [ BOTDEFENSE::device_id] from IP [ IP::client_addr ] visited [HTTP::uri ]"\n'
                    "}"
                ),
                return_value="The name assigned to the bot, browser or mobile application that sent the request.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::bot_name",
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
