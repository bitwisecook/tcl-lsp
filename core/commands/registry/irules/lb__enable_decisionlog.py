# Enriched from F5 iRules reference documentation.
"""LB::enable_decisionlog -- Enables LTM decision logging"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__enable_decisionlog.html"


@register
class LbEnableDecisionlogCommand(CommandDef):
    name = "LB::enable_decisionlog"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::enable_decisionlog",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables LTM decision logging",
                synopsis=("LB::enable_decisionlog",),
                snippet="Enables LTM decision logging",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::enable_decisionlog",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"FASTHTTP"}),
                also_in=frozenset({"CLIENT_ACCEPTED"}),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LOG_IO, writes=True, connection_side=ConnectionSide.BOTH
                ),
            ),
        )
