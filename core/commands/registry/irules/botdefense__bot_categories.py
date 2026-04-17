# Enriched from F5 iRules reference documentation.
"""BOTDEFENSE::bot_categories -- Returns the list of category names to which the current client belongs."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__bot_categories.html"


@register
class BotdefenseBotCategoriesCommand(CommandDef):
    name = "BOTDEFENSE::bot_categories"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::bot_categories",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the list of category names to which the current client belongs.",
                synopsis=("BOTDEFENSE::bot_categories",),
                snippet="Returns the list of category names to which the current client belongs. These categories are determined by the anomalies found for the respective client. Note these categories are additional to the bot signature category which is applicable if a bot signature was found.",
                source=_SOURCE,
                examples=(
                    "when BOTDEFENSE_ACTION {\n"
                    "    foreach {cat} [BOTDEFENSE::bot_categories] {\n"
                    '        log.local0. "Found category: $cat"\n'
                    "    }\n"
                    "}"
                ),
                return_value="Returns a list of all category names to which the current client belongs based on the anomalies found for the client. The categories come in addition to the bot signature category optionally detected and returned in BOTDEFENSE::bot_signature_category. If no anomaly found then the list will be empty.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::bot_categories",
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
