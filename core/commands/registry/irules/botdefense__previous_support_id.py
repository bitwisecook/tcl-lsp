# Enriched from F5 iRules reference documentation.
"""BOTDEFENSE::previous_support_id -- Returns the Device ID of the client, as retrieved from the request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__previous_support_id.html"


@register
class BotdefensePreviousSupportIdCommand(CommandDef):
    name = "BOTDEFENSE::previous_support_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::previous_support_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the Device ID of the client, as retrieved from the request.",
                synopsis=("BOTDEFENSE::previous_support_id",),
                snippet='Returns the Support ID of the previous request; this is applicable if the current request is a follow-up to a challenge. Otherwise, "0" is returned.',
                source=_SOURCE,
                examples=(
                    "# EXAMPLE: Log the Support ID of the previous request.\n"
                    "when BOTDEFENSE_REQUEST {\n"
                    '    set log "botdefense previous support ID is"\n'
                    '    append log " [BOTDEFENSE::previous_support_id]"\n'
                    "    HSL::send $hsl $log\n"
                    "}"
                ),
                return_value="Returns the support ID of the previous request, or 0 if not applicable.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::previous_support_id",
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
