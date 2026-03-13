# Enriched from F5 iRules reference documentation.
"""BOTDEFENSE::support_id -- Returns the support ID of the request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__support_id.html"


@register
class BotdefenseSupportIdCommand(CommandDef):
    name = "BOTDEFENSE::support_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::support_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the support ID of the request.",
                synopsis=("BOTDEFENSE::support_id",),
                snippet="Returns a number, representing the support ID of the request.",
                source=_SOURCE,
                examples=(
                    "# EXAMPLE: Log the support ID of the request.\n"
                    "when BOTDEFENSE_REQUEST {\n"
                    '    set log "support id is"\n'
                    '    append log " [BOTDEFENSE::support_id]"\n'
                    "    HSL::send $hsl $log\n"
                    "}"
                ),
                return_value="Returns the support ID of the reuqest.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::support_id",
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
