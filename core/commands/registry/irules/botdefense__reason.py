# Enriched from F5 iRules reference documentation.
"""BOTDEFENSE::reason -- Returns the reason for the Bot Defense action."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__reason.html"


@register
class BotdefenseReasonCommand(CommandDef):
    name = "BOTDEFENSE::reason"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::reason",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the reason for the Bot Defense action.",
                synopsis=("BOTDEFENSE::reason",),
                snippet="Returns the reason that lead Bot Defense to decide on the action to be taken (the action that is specified in BOTDEFENSE::action).",
                source=_SOURCE,
                examples=(
                    "# EXAMPLE: Send the Bot Defense action and reason to High Speed Logging\n"
                    "when BOTDEFENSE_ACTION {\n"
                    '    HSL::send $hsl "action [BOTDEFENSE::action] reason \\"[BOTDEFENSE::reason]\\""\n'
                    "}"
                ),
                return_value="Returns a string signifying the reason for the Bot Defense action.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::reason",
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
