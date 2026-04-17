# Enriched from F5 iRules reference documentation.
"""TAP::insight -- Accumulates or sends key:value pairs to TAP, returns token."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TAP__insight.html"


@register
class TapInsightCommand(CommandDef):
    name = "TAP::insight"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TAP::insight",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Accumulates or sends key:value pairs to TAP, returns token.",
                synopsis=(
                    "TAP::insight set (TAP_INSIGHT_KEY TAP_INSIGHT_VALUE)*",
                    "TAP::insight send TAP_INSIGHT_EVENT_TYPE TAP_INSIGHT_REASON",
                ),
                snippet=(
                    "With arguments accumulates them as key:value pairs, without arguments sends accumulated to TAP.\n"
                    "Returns token supplied by TAP service."
                ),
                source=_SOURCE,
                examples=(
                    "when TAP_REQUEST {\n"
                    '    if { ([TAP::insight] eq "block") } {\n'
                    "        drop\n"
                    "    }\n"
                    "}"
                ),
                return_value="Returns one of the following actions: allow, block, captcha, conviction, deception, timeout.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TAP::insight set (TAP_INSIGHT_KEY TAP_INSIGHT_VALUE)*",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
