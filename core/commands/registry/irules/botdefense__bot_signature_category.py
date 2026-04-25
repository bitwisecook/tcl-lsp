# Enriched from F5 iRules reference documentation.
"""BOTDEFENSE::bot_signature_category -- Returns the name of the detected Bot Signature Category."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__bot_signature_category.html"


@register
class BotdefenseBotSignatureCategoryCommand(CommandDef):
    name = "BOTDEFENSE::bot_signature_category"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::bot_signature_category",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the name of the detected Bot Signature Category.",
                synopsis=("BOTDEFENSE::bot_signature_category",),
                snippet="Returns the name of the detected Bot Signature Category, or an empty string if no bot signature was detected.",
                source=_SOURCE,
                examples=(
                    "# EXAMPLE: Log the bot signature category.\n"
                    "when BOTDEFENSE_REQUEST {\n"
                    '    set log "botdefense bot_signature_category is"\n'
                    '    append log " [BOTDEFENSE::bot_signature_category]"\n'
                    "    HSL::send $hsl $log\n"
                    "}"
                ),
                return_value="Returns the name of the detected Bot Signature Category, or an empty string if no bot signature was detected.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::bot_signature_category",
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
